//! When the hints are told what (S67): reports while a hand is on or over the glass and for the
//! short while after, one pause once it rests, one close on leaving, nothing without a host.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use openpaint_app::{Contact, ContactPhase, Device, InputBatch, InputSample};
use openpaint_render::FrameTime;

use super::{FrameWork, Hints, WorkHints};
use crate::touches::{IDLE_AFTER, Touches};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Call {
    Joined,
    Report(FrameWork),
    Pause,
    Close,
}

#[derive(Default)]
struct Recorded(Mutex<Vec<Call>>);

impl Recorded {
    fn take(&self) -> Vec<Call> {
        std::mem::take(&mut self.0.lock().expect("calls"))
    }
}

impl WorkHints for Recorded {
    fn joined(&self) {
        self.0.lock().expect("calls").push(Call::Joined);
    }
    fn report(&self, work: FrameWork) {
        self.0.lock().expect("calls").push(Call::Report(work));
    }
    fn pause(&self) {
        self.0.lock().expect("calls").push(Call::Pause);
    }
    fn close(&self) {
        self.0.lock().expect("calls").push(Call::Close);
    }
}

fn pen(phase: ContactPhase) -> InputBatch {
    InputBatch {
        contacts: vec![Contact {
            id: 0,
            device: Device::Pen,
            phase,
            samples: vec![InputSample {
                position: [10.0, 10.0],
                pressure: 0.5,
                tilt: 0.0,
                azimuth: 0.0,
                time_ms: 0,
            }],
        }],
        ..InputBatch::default()
    }
}

/// A frame call at `now` with the screen as `touches` sees it, drawn: what the host was told.
fn frame(hints: &mut Hints, touches: &mut Touches, now: Instant, host: &Recorded) -> Vec<Call> {
    let idle = touches.idle_at(false, now);
    hints.called(idle, Some((now, Duration::from_millis(6))));
    host.take()
}

fn reports(calls: &[Call]) -> usize {
    calls
        .iter()
        .filter(|call| matches!(call, Call::Report(_)))
        .count()
}

#[test]
fn reports_while_a_hand_is_on_or_over_the_glass_and_a_while_after_then_pauses_once() {
    let host = Arc::new(Recorded::default());
    let (mut hints, mut touches) = (Hints::default(), Touches::default());
    hints.set(host.clone());
    let start = Instant::now();
    assert!(
        frame(&mut hints, &mut touches, start, &host).is_empty(),
        "nothing touched yet"
    );
    assert!(!hints.timing(true), "frames untimed at rest");
    touches.saw(&[pen(ContactPhase::Hover)]);
    let hover = frame(&mut hints, &mut touches, Instant::now(), &host);
    assert_eq!(reports(&hover), 1, "the pen hovering: {hover:?}");
    assert!(hints.timing(false), "frames timed while a hand is there");
    touches.saw(&[pen(ContactPhase::Down)]);
    assert_eq!(
        reports(&frame(&mut hints, &mut touches, Instant::now(), &host)),
        1
    );
    touches.saw(&[pen(ContactPhase::Up), pen(ContactPhase::Leave)]);
    let left = Instant::now();
    let tail = frame(&mut hints, &mut touches, left + IDLE_AFTER / 2, &host);
    assert_eq!(reports(&tail), 1, "the short while after: {tail:?}");
    let rest = frame(&mut hints, &mut touches, left + IDLE_AFTER * 2, &host);
    assert_eq!(rest, [Call::Pause], "the screen rests");
    let later = frame(&mut hints, &mut touches, left + IDLE_AFTER * 3, &host);
    assert!(later.is_empty(), "paused once, nothing more: {later:?}");
    touches.saw(&[pen(ContactPhase::Hover)]);
    assert_eq!(
        reports(&frame(&mut hints, &mut touches, Instant::now(), &host)),
        1,
        "back"
    );
}

#[test]
fn a_skipped_call_reports_nothing_and_a_report_carries_the_latest_gpu_frame() {
    let host = Arc::new(Recorded::default());
    let mut hints = Hints::default();
    hints.set(host.clone());
    hints.called(false, None);
    assert!(host.take().is_empty(), "no frame drawn");
    let began = Instant::now();
    hints.called(false, Some((began, Duration::from_millis(6))));
    let [Call::Report(untimed)] = host.take()[..] else {
        panic!("one report");
    };
    assert_eq!(
        (untimed.gpu, untimed.total),
        (None, Duration::from_millis(6))
    );
    hints.read(&[
        FrameTime {
            gpu_ns: 3e6,
            done_ns: 8e6,
        },
        FrameTime {
            gpu_ns: 4e6,
            done_ns: 10e6,
        },
    ]);
    hints.called(false, Some((began, Duration::from_millis(6))));
    let [Call::Report(timed)] = host.take()[..] else {
        panic!("one report");
    };
    assert_eq!(timed.gpu, Some(Duration::from_millis(4)), "the latest");
    assert_eq!(
        timed.total,
        Duration::from_millis(10),
        "until the GPU finished it"
    );
    assert_eq!(timed.began, began);
}

#[test]
fn leaving_closes_once_and_a_hand_after_starts_again() {
    let host = Arc::new(Recorded::default());
    let mut hints = Hints::default();
    hints.set(host.clone());
    hints.leave();
    assert!(host.take().is_empty(), "never reported: nothing to close");
    hints.called(false, Some((Instant::now(), Duration::from_millis(5))));
    hints.leave();
    hints.leave();
    let calls = host.take();
    assert_eq!(reports(&calls), 1);
    assert_eq!(calls.last(), Some(&Call::Close), "closed once: {calls:?}");
    assert_eq!(calls.iter().filter(|c| **c == Call::Close).count(), 1);
    hints.called(true, None);
    assert!(host.take().is_empty(), "closed: no pause");
    hints.called(false, Some((Instant::now(), Duration::from_millis(5))));
    assert_eq!(
        reports(&host.take()),
        1,
        "back in front, a hand: reports again"
    );
}

#[test]
fn without_hints_from_the_host_nothing_is_told_or_timed() {
    let mut hints = Hints::default();
    assert!(!hints.timing(false), "no frames timed for hints");
    hints.called(false, Some((Instant::now(), Duration::from_millis(5))));
    hints.called(true, None);
    hints.leave();
    let host = Recorded::default();
    host.joined();
    assert_eq!(host.take(), [Call::Joined], "the recorder records");
}
