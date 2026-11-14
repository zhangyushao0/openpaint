//! The recorder of frames in real use on the hand's held clock: a stroke's run of frames is one
//! line saying what was drawn of the vsyncs that wanted a frame (one missed, one skipped for the
//! GPU); frames the host asks for at rest, a second apart, and a run shorter than a quarter of a
//! second are none.

use std::time::Instant;

use openpaint_runtime::{
    Buttons, FRAMES_FILE, FrameRecorder, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime,
    ToolKind,
};

use crate::support::hand::HANG;
use crate::support::{Nowhere, TestDir, settle, start};

const SIZE: [u32; 2] = [1280, 1000];

/// The held clock at vsync `k` (120 Hz) after `base`, ms.
fn at(base: i64, k: i64) -> i64 {
    base + (k as f64 * 1000.0 / 120.0).round() as i64
}

/// A pen batch of `phase` at `x`, its sample stamped `time_ms`.
fn pen(phase: MotionPhase, x: f32, time_ms: i64) -> MotionBatch {
    MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id: 0,
            tool: ToolKind::Stylus,
            samples: vec![PointerSample {
                position: [x, 500.0],
                pressure: 0.8,
                tilt: 0.0,
                orientation: 0.0,
                time_ms,
            }],
        }],
    }
}

/// What the frame calls of a run did: calls, those skipped for the GPU, those made while the pen
/// touched, the vsync after the last.
#[derive(Debug, Default)]
struct Run {
    calls: u32,
    skipped: u32,
    touched: u32,
    next: i64,
}

impl Run {
    /// A frame call at vsync `k`, as the host makes it; whether the runtime wants the next.
    fn call(&mut self, runtime: &mut Runtime, recorder: &mut FrameRecorder, k: i64) -> bool {
        // Choreographer delivers the vsync of a 120 Hz panel.
        recorder.delivered(k * 8_333_333);
        let again = runtime.frame_recorded(recorder).expect("frame");
        self.calls += 1;
        self.skipped += u32::from(runtime.paced().skipped);
        self.next = k + 1;
        again
    }

    /// Frame calls at the next vsyncs while the runtime wants them, as the host makes them.
    fn follow(&mut self, runtime: &mut Runtime, recorder: &mut FrameRecorder, base: i64) {
        let started = Instant::now();
        loop {
            runtime.fix_clock(at(base, self.next));
            let k = self.next;
            if !self.call(runtime, recorder, k) {
                return;
            }
            assert!(
                started.elapsed() < HANG,
                "frames still wanted after {HANG:?}"
            );
        }
    }
}

/// A pen stroke over vsyncs `from..from + vsyncs` after `base`: a batch at each, followed at once
/// by a frame call but at `without` (that vsync's call missed: the next comes a vsync late); the
/// GPU held behind for the call at `held`; then frames while the runtime wants them.
fn stroke(
    (runtime, recorder): (&mut Runtime, &mut FrameRecorder),
    (base, from, vsyncs): (i64, i64, i64),
    (without, held): (Option<i64>, Option<i64>),
) -> Run {
    let mut run = Run::default();
    let mut again = false;
    for k in from..from + vsyncs {
        let phase = match k - from {
            0 => MotionPhase::Down,
            n if n == vsyncs - 1 => MotionPhase::Up,
            _ => MotionPhase::Move,
        };
        runtime.fix_clock(at(base, k));
        let x = 200.0 + 10.0 * (k - from) as f32;
        runtime.motion_recorded(&pen(phase, x, at(base, k)), recorder);
        if Some(k) == held {
            runtime.hold_gpu(1);
        }
        if Some(k) != without {
            again = run.call(runtime, recorder, k);
        }
    }
    run.touched = run.calls;
    if again {
        run.follow(runtime, recorder, base);
    }
    run
}

/// Frame calls nobody asked for at `count` vsyncs from `from` after `base` (a runtime at rest
/// wants none after them).
fn frames(
    runtime: &mut Runtime,
    recorder: &mut FrameRecorder,
    (base, from, count): (i64, i64, i64),
) -> Run {
    let mut run = Run::default();
    let mut again = false;
    for k in from..from + count {
        runtime.fix_clock(at(base, k));
        again = run.call(runtime, recorder, k);
    }
    if again {
        run.follow(runtime, recorder, base);
    }
    run
}

/// The lines the recorder kept, once it is dropped (its worker writes what it was handed).
fn lines(dir: &TestDir, recorder: FrameRecorder) -> Vec<String> {
    drop(recorder);
    let text = std::fs::read_to_string(dir.join(FRAMES_FILE)).unwrap_or_default();
    text.lines().map(str::to_owned).collect()
}

/// A runtime settled on its canvas, the hand's clock a second on, and a recorder on `dir`.
fn recording(dir: &TestDir) -> (Runtime, i64, FrameRecorder) {
    let (mut runtime, mut hand) = start(2.0, SIZE, Box::new(Nowhere::default()));
    settle(&mut runtime, &mut hand);
    let recorder = FrameRecorder::writing_to(dir.join(FRAMES_FILE)).expect("a recorder");
    (runtime, hand.clock + 1000, recorder)
}

#[test]
fn a_run_of_frames_wanted_is_one_line_counting_a_missed_vsync_and_a_skipped_call() {
    let dir = TestDir::new("frames-stroke");
    let (mut runtime, base, mut recorder) = recording(&dir);
    let run = stroke(
        (&mut runtime, &mut recorder),
        (base, 0, 50),
        (Some(25), Some(10)),
    );
    // Long after, a frame call nobody asked for: the stretch ended before it.
    runtime.fix_clock(at(base, run.next + 60));
    runtime.frame_recorded(&mut recorder).expect("frame");
    let lines = lines(&dir, recorder);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(run.skipped >= 1, "the GPU held behind skipped a call");
    let (wanted, drawn) = (run.calls + 1, run.calls - run.skipped);
    assert_eq!(
        i64::from(wanted),
        run.next,
        "every vsync from the pen's touch on"
    );
    let percent = 100.0 * f64::from(drawn) / f64::from(wanted);
    let counted = format!(" drawn {drawn}/{wanted} {percent:.0}% work ");
    assert!(lines[0].contains(&counted), "{} lacks{counted}", lines[0]);
    // The hand of most of its frame calls: the pen's, unless the frames after the lift (the
    // store's work, its worker in real time against the held clock) outnumber them.
    let after = run.calls - run.touched;
    let hand = if run.touched >= after { "pen" } else { "none" };
    assert!(lines[0].contains(" display 120Hz "), "{}", lines[0]);
    let shown = format!(" hand {hand} screen canvas");
    assert!(lines[0].ends_with(&shown), "{} lacks{shown}", lines[0]);
}

#[test]
fn frames_at_rest_and_a_run_shorter_than_a_quarter_second_are_no_line() {
    let dir = TestDir::new("frames-rest");
    let (mut runtime, base, mut recorder) = recording(&dir);
    // At rest: a frame call each second, as the Pomodoro's tick asks for one.
    for second in 0..10 {
        frames(&mut runtime, &mut recorder, (base, 120 * second, 1));
    }
    // A flicker: 29 vsyncs in a row wanting a frame; then, apart, 30: a quarter of a second.
    let flicker = frames(&mut runtime, &mut recorder, (base, 2000, 29));
    assert_eq!(
        flicker.next, 2029,
        "a runtime at rest wants no frame after them"
    );
    let kept = frames(&mut runtime, &mut recorder, (base, 2100, 30));
    assert_eq!(kept.next, 2130);
    recorder.end();
    let lines = lines(&dir, recorder);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(" 0.25s drawn "), "{}", lines[0]);
    assert!(
        lines[0].ends_with(" hand none screen canvas"),
        "{}",
        lines[0]
    );
}
