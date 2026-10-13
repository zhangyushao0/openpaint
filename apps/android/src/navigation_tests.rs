use super::*;
use openpaint_runtime::{Buttons, Pointer, PointerSample};

const MS: i64 = 1_000_000;

fn batch(phase: MotionPhase, tools: &[ToolKind]) -> MotionBatch {
    let sample = PointerSample {
        position: [0.0, 0.0],
        pressure: 1.0,
        tilt: 0.0,
        orientation: 0.0,
        time_ms: 0,
    };
    MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: (0i32..)
            .zip(tools)
            .map(|(id, tool)| Pointer {
                id,
                tool: *tool,
                samples: vec![sample],
            })
            .collect(),
    }
}

/// A frame starting at `start` ms for `work` ms, its background 2 ms by the corrected estimate,
/// 1.5 measured.
fn frame(start: i64, work: i64, waiting: bool, entries: u64) -> Frame {
    Frame {
        start_ns: start * MS,
        work_ns: work * MS,
        waiting,
        counts: Counts {
            entries,
            ..Counts::default()
        },
        background: Background {
            spent: 2e6,
            ratio: 0.5,
            measured: Some(1.5e6),
            pieces: 3,
            first: 1e6,
        },
    }
}

#[test]
fn an_episode_runs_while_fingers_are_down_or_work_waits() {
    let mut log = NavigationLog::default();
    let finger = ToolKind::Finger;
    assert_eq!(log.frame(frame(0, 1, false, 0)), None, "idle: nothing");
    log.input(&batch(MotionPhase::Down, &[finger]));
    log.input(&batch(
        MotionPhase::PointerDown { index: 1 },
        &[finger, finger],
    ));
    assert_eq!(log.frame(frame(10, 4, false, 5)), None);
    log.input(&batch(
        MotionPhase::PointerUp { index: 1 },
        &[finger, finger],
    ));
    log.input(&batch(MotionPhase::Up, &[finger]));
    // The fingers lifted; a tier still fills.
    assert_eq!(log.frame(frame(20, 12, true, 9)), None);
    assert_eq!(log.frame(frame(30, 2, true, 12)), None);
    let summary = log.frame(frame(40, 1, false, 20)).expect("ended");
    assert_eq!(
        (summary.frames, summary.touched, summary.span_ms),
        (3, 1, 30)
    );
    assert_eq!(summary.work.max, 12 * MS);
    assert_eq!(summary.background.p50, 2 * MS, "the corrected estimate");
    assert!((summary.measured_ms - 4.5).abs() < 1e-9);
    assert_eq!(
        summary.counts.entries, 12,
        "from before its first frame to its last"
    );
    assert_eq!(log.frame(frame(50, 1, false, 20)), None, "once");
}

#[test]
fn a_pen_is_no_finger_and_long_episodes_start_over() {
    let mut log = NavigationLog::default();
    log.input(&batch(MotionPhase::Down, &[ToolKind::Stylus]));
    assert_eq!(log.frame(frame(0, 1, false, 0)), None);
    assert!(log.frame(frame(1, 1, true, 0)).is_none());
    let long = LONGEST / MS + 2;
    let summary = log.frame(frame(long, 1, true, 3));
    assert_eq!(summary.map(|s| s.frames), Some(1));
    let line = log.frame(frame(long + 1, 1, false, 4)).expect("ended");
    assert!(
        line.to_string()
            .starts_with("navigation, 1 frames (0 with fingers)")
    );
}
