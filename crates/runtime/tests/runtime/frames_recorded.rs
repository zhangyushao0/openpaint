//! The recorder of frames in real use on the hand's held clock: a stroke's run of frames is one
//! line saying what was drawn of the vsyncs a hand or a motion wanted a frame at (one missed, one
//! skipped for the GPU), ended by the first frame call at rest after it; frame calls at rest,
//! however many in a row, and a run shorter than a quarter of a second are none.

use std::time::Instant;

use openpaint_runtime::{
    Buttons, FRAMES_FILE, FrameRecorder, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime,
    ToolKind, Wanted,
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

/// What the frame calls of a run did up to the first wanted at rest (`rested`, its vsync): calls,
/// those skipped for the GPU, those made while the pen touched; the vsync after the last call.
#[derive(Debug, Default)]
struct Run {
    calls: u32,
    skipped: u32,
    touched: u32,
    rested: Option<i64>,
    /// Calls wanted at rest, all told.
    at_rest: u32,
    next: i64,
}

impl Run {
    /// A frame call at vsync `k`, as the host makes it; whether the runtime wants the next.
    fn call(&mut self, runtime: &mut Runtime, recorder: &mut FrameRecorder, k: i64) -> bool {
        // Choreographer delivers the vsync of a 120 Hz panel.
        recorder.delivered(k * 8_333_333);
        let again = runtime.frame_recorded(recorder).expect("frame");
        let paced = runtime.paced();
        if paced.wanted == Wanted::Rest {
            self.at_rest += 1;
            self.rested.get_or_insert(k);
        } else if self.rested.is_none() {
            self.calls += 1;
            self.skipped += u32::from(paced.skipped);
        }
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

/// A pen stroke's phases: down, moving, up.
const STROKE: [MotionPhase; 3] = [MotionPhase::Down, MotionPhase::Move, MotionPhase::Up];
/// A pen hovering over the glass and leaving it.
const HOVER: [MotionPhase; 3] = [
    MotionPhase::HoverEnter,
    MotionPhase::HoverMove,
    MotionPhase::HoverExit,
];

/// A pen gesture of `phases` (`STROKE` or `HOVER`) over vsyncs `from..from + vsyncs` after `base`:
/// a batch at each, followed at once by a frame call but at `without` (that vsync's call missed:
/// the next comes a vsync late); the GPU held behind for the call at `held`; then frames while
/// the runtime wants them.
fn stroke(
    (runtime, recorder): (&mut Runtime, &mut FrameRecorder),
    (base, from, vsyncs): (i64, i64, i64),
    (without, held): (Option<i64>, Option<i64>),
    [first, between, last]: [MotionPhase; 3],
) -> Run {
    let mut run = Run::default();
    let mut again = false;
    for k in from..from + vsyncs {
        let phase = match k - from {
            0 => first,
            n if n == vsyncs - 1 => last,
            _ => between,
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
        STROKE,
    );
    // Long after, a frame call nobody asked for: the stretch ended before it.
    runtime.fix_clock(at(base, run.next + 60));
    runtime.frame_recorded(&mut recorder).expect("frame");
    let lines = lines(&dir, recorder);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        run.skipped >= 1,
        "the GPU held behind skipped a call the stroke wanted"
    );
    let (wanted, drawn) = (run.calls + 1, run.calls - run.skipped);
    assert_eq!(
        i64::from(wanted),
        run.rested.unwrap_or(run.next),
        "every vsync from the pen's touch to the first call at rest"
    );
    let percent = 100.0 * f64::from(drawn) / f64::from(wanted);
    let counted = format!(" drawn {drawn}/{wanted} {percent:.0}% work ");
    assert!(lines[0].contains(&counted), "{} lacks{counted}", lines[0]);
    // The hand of most of its frame calls: the pen's, unless the frames a motion wanted after the
    // lift outnumber them.
    let after = run.calls - run.touched;
    let hand = if run.touched >= after { "pen" } else { "none" };
    assert!(lines[0].contains(" display 120Hz "), "{}", lines[0]);
    let shown = format!(" hand {hand} screen canvas");
    assert!(lines[0].ends_with(&shown), "{} lacks{shown}", lines[0]);
}

#[test]
fn calls_at_rest_are_no_stretch_and_a_run_shorter_than_a_quarter_second_no_line() {
    let dir = TestDir::new("frames-rest");
    let (mut runtime, base, mut recorder) = recording(&dir);
    // At rest: a frame call each second, as the Pomodoro's tick asks for one, then a run of them
    // a vsync apart, as background work asks for them: none was owed a frame.
    for second in 0..10 {
        frames(&mut runtime, &mut recorder, (base, 120 * second, 1));
    }
    let resting = frames(&mut runtime, &mut recorder, (base, 1500, 60));
    assert_eq!((resting.calls, resting.at_rest), (0, 60), "{resting:?}");
    // A flicker: the pen hovering over 29 vsyncs; then, apart, over 30: a quarter of a second.
    let mut hover = |from, vsyncs| {
        let run = stroke(
            (&mut runtime, &mut recorder),
            (base, from, vsyncs),
            (None, None),
            HOVER,
        );
        assert_eq!(i64::from(run.calls), vsyncs, "{run:?}");
        assert_eq!(run.rested, Some(from + vsyncs), "at rest once it left");
    };
    hover(2000, 29);
    hover(2100, 30);
    recorder.end();
    let lines = lines(&dir, recorder);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(" 0.25s drawn 30/30 "), "{}", lines[0]);
    assert!(
        lines[0].ends_with(" hand none screen canvas"),
        "{}",
        lines[0]
    );
}
