use super::*;
use openpaint_runtime::{Buttons, Pointer, PointerSample};

const MS: i64 = 1_000_000;

fn batch(phase: MotionPhase, tool: ToolKind) -> MotionBatch {
    MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id: 0,
            tool,
            samples: vec![PointerSample {
                position: [0.0, 0.0],
                pressure: 0.5,
                tilt: 0.0,
                orientation: 0.0,
                time_ms: 0,
            }],
        }],
    }
}

fn cost(waited_ms: i64, scene_ms: i64, in_flight: u64) -> FrameCost {
    FrameCost {
        drawn_ns: 2 * scene_ms * MS,
        scene_ns: scene_ms * MS,
        waited_ns: waited_ms * MS,
        longest: if waited_ms > 0 { "present" } else { "" },
        in_flight,
        ..FrameCost::default()
    }
}

fn drew(regions: u32, snapshots: u32, laid_again: u32) -> StrokeDrew {
    StrokeDrew {
        regions,
        snapshots,
        laid_again,
        ..StrokeDrew::NONE
    }
}

/// A frame drawing the canvas for `canvas_ms` and the tier for `tier_ms`, drawing `canvas` and
/// `tier`.
fn painting(canvas_ms: i64, tier_ms: i64, (canvas, tier): (StrokeDrew, StrokeDrew)) -> FrameCost {
    FrameCost {
        drawn_ns: canvas_ms * MS,
        tier_ns: tier_ms * MS,
        canvas,
        tier,
        ..FrameCost::default()
    }
}

#[test]
fn a_stroke_is_summarised_when_the_frame_takes_its_lift() {
    let mut latency = PenLatency::default();
    latency.input(&batch(MotionPhase::Down, ToolKind::Stylus), 0);
    assert_eq!(latency.frame((4 * MS, 6 * MS), cost(1, 1, 0)), None);
    latency.input(&batch(MotionPhase::Move, ToolKind::Stylus), 10 * MS);
    latency.skipped();
    latency.input(&batch(MotionPhase::Move, ToolKind::Stylus), 12 * MS);
    assert_eq!(latency.frame((20 * MS, 23 * MS), cost(2, 0, 1)), None);
    latency.input(&batch(MotionPhase::Up, ToolKind::Stylus), 30 * MS);
    let summary = latency
        .frame((36 * MS, 38 * MS), cost(0, 1, 1))
        .expect("summary");
    assert_eq!((summary.events, summary.frames), (4, 3));
    assert_eq!((summary.skipped, summary.in_flight), (1, 1));
    // To frame start: 4, 10, 8, 6 ms; to present call: 6, 13, 11, 8; work: 2, 3, 2.
    let spread = |p50, p95, max| Spread {
        p50: p50 * MS,
        p95: p95 * MS,
        max: max * MS,
    };
    assert_eq!(summary.to_frame, spread(6, 10, 10));
    assert_eq!(summary.to_present, spread(8, 13, 13));
    assert_eq!(summary.frame_work, spread(2, 3, 3));
    assert_eq!(summary.waited, spread(1, 2, 2));
    assert_eq!(summary.painted, None, "nothing drawn: no painting fields");
    let slow = |at, ms, cost| Slow {
        at,
        ns: ms * MS,
        cost,
    };
    assert_eq!(
        summary.slowest,
        [
            slow(1, 3, cost(2, 0, 1)),
            slow(0, 2, cost(1, 1, 0)),
            slow(2, 2, cost(0, 1, 1))
        ]
    );
    let line = summary.to_string();
    let paced = "(1 skipped for the GPU, at most 1 in flight; the frame before still in flight";
    assert!(
        line.contains(&format!("{paced} at 3 of 4 calls)")),
        "{line}"
    );
    assert!(
        line.contains("GPU waits p50 1.0 p95 2.0 max 2.0 ms; slowest #1"),
        "{line}"
    );
    assert!(
        line.ends_with(
            "slowest #1 3.0 (evaluation 0.0, tier 0.0, chains canvas 0 tier 0, scene 0.0, waits \
             2.0 most in present) #0 2.0 (evaluation 2.0, tier 0.0, chains canvas 0 tier 0, \
             scene 1.0, waits 1.0 most in present) #2 2.0 (evaluation 2.0, tier 0.0, chains \
             canvas 0 tier 0, scene 1.0, waits 0.0) ms"
        ),
        "{line}"
    );
    assert_eq!(
        latency.frame((50 * MS, 52 * MS), FrameCost::default()),
        None,
        "one summary per stroke"
    );
}

/// At a tier the canvas draws a frame only where the tier could not, and the lift: the line says
/// each one's drawing time, chains, snapshots and strokes laid again.
#[test]
fn a_painting_stroke_says_where_its_frames_went() {
    let mut latency = PenLatency::default();
    let stylus = |phase| batch(phase, ToolKind::Stylus);
    latency.input(&stylus(MotionPhase::Down), 0);
    let first = (drew(0, 0, 0), drew(2, 2, 0));
    assert_eq!(latency.frame((1, 2), painting(1, 6, first)), None);
    // A frame consuming no event still counts what it drew over the stroke.
    let quiet = (drew(0, 0, 0), drew(1, 1, 0));
    assert_eq!(latency.frame((3, 4), painting(0, 1, quiet)), None);
    latency.input(&stylus(MotionPhase::Move), 5);
    let left = (drew(1, 1, 0), drew(1, 0, 3));
    assert_eq!(latency.frame((6, 7), painting(4, 3, left)), None);
    latency.input(&stylus(MotionPhase::Move), 8);
    let tier = (drew(0, 0, 0), drew(4, 0, 3));
    assert_eq!(latency.frame((9, 10), painting(1, 2, tier)), None);
    latency.input(&stylus(MotionPhase::Up), 11);
    let lift = (drew(2, 0, 5), drew(1, 0, 0));
    let summary = latency
        .frame((12, 13), painting(5, 1, lift))
        .expect("summary");
    let painted = summary.painted.expect("it painted");
    let spread = |p50, p95, max| Spread { p50, p95, max };
    // Evaluation 1, 4, 1, 5 ms; tier 6, 3, 2, 1; chains canvas 0, 1, 0, 2 and tier 2, 1, 4, 1.
    assert_eq!(painted.evaluation, spread(MS, 5 * MS, 5 * MS));
    assert_eq!(painted.tier_ns, spread(2 * MS, 6 * MS, 6 * MS));
    // The tier at one chain 3 and 1 ms, at more 6 and 2.
    assert_eq!(painted.tier_one, (spread(MS, 3 * MS, 3 * MS), 2));
    assert_eq!(painted.tier_more, (spread(2 * MS, 6 * MS, 6 * MS), 2));
    assert_eq!(painted.canvas_chains, spread(0, 2, 2));
    assert_eq!(painted.tier_chains, spread(1, 4, 4));
    assert_eq!(painted.canvas_frames, 2);
    assert_eq!(
        (painted.canvas, painted.tier),
        (drew(3, 1, 5), drew(9, 3, 6))
    );
    let line = summary.to_string();
    assert!(
        line.contains(
            "GPU waits p50 0.0 p95 0.0 max 0.0 ms; evaluation p50 1.0 p95 5.0, tier p50 2.0 p95 \
             6.0 ms (one chain p50 1.0 p95 3.0 in 2 frames, more p50 2.0 p95 6.0 in 2); chains \
             a frame canvas p50 0 max 2 (drawn in 2 frames), tier p50 1 max 4; snapshots taken \
             canvas 1, tier 3; strokes laid again canvas 5, tier 6; render passes a frame p50 0 \
             max 0, their attachments loaded p50 0.0 max 0.0 MiB, stored p50 0.0 max 0.0 MiB; \
             a one-chain frame's drawing \
             p50 track 0.00, ops 0.00, plan 0.00, encode 0.00, writes 0.00, submit 0.00 ms in 0 \
             frames; beside the drawing and its waits p50 UI pass 0.0, the rest 0.0 ms; slowest #0"
        ),
        "{line}"
    );
    assert!(
        line.contains("#0 0.0 (evaluation 1.0, tier 6.0, chains canvas 0 tier 2, scene 0.0"),
        "each slowest frame names its chains: {line}"
    );
}

/// A frame whose drawing was one pass chain says where that drawing went, part by part; every
/// frame what it spent beside the drawing and its waits: its UI pass and the rest (the drawing's
/// own submissions, among its waits too, counted once).
#[test]
fn a_one_chain_frame_says_where_its_drawing_went_and_every_frame_the_rest() {
    let mut latency = PenLatency::default();
    let stylus = |phase| batch(phase, ToolKind::Stylus);
    let tenth = MS as u64 / 10;
    let frame = |k: u64, chains: u32, (ui_ms, waited_ms): (i64, i64)| FrameCost {
        ui_ns: ui_ms * MS,
        tier_ns: 3 * MS,
        waited_ns: waited_ms * MS,
        tier: StrokeDrew {
            regions: chains,
            parts: StrokeParts {
                track_ns: k * tenth,
                ops_ns: 2 * k * tenth,
                plan_ns: 3 * k * tenth,
                encode_ns: 10 * k * tenth,
                writes_ns: 4 * k * tenth,
                submit_ns: 5 * k * tenth,
            },
            // Its passes and what they loaded and stored: k MiB and twice that.
            attached: Attached {
                passes: 3 + chains,
                loaded: k << 20,
                stored: (2 * k) << 20,
            },
            ..StrokeDrew::NONE
        },
        ..FrameCost::default()
    };
    latency.input(&stylus(MotionPhase::Down), 0);
    // Work 8 ms: UI 2, tier 3, waits 1 less its submission 0.5: the rest 2.5.
    latency.frame((0, 8 * MS), frame(1, 1, (2, 1)));
    latency.input(&stylus(MotionPhase::Move), 9 * MS);
    // Work 9: UI 3, tier 3, waits 2 less 1: the rest 2.
    latency.frame((10 * MS, 19 * MS), frame(2, 1, (3, 2)));
    latency.input(&stylus(MotionPhase::Up), 19 * MS);
    // Two chains, so not among the parts; work 10: UI 2, tier 3, waits 3 less 1.5: the rest 3.5.
    let summary = (latency.frame((20 * MS, 30 * MS), frame(3, 2, (2, 3)))).expect("summary");
    let painted = summary.painted.expect("it painted");
    let tenths = [1, 2, 3, 10, 4, 5].map(|k| k * MS / 10);
    assert_eq!(painted.one_chain, (tenths, 2), "the first frame's at p50");
    assert_eq!((painted.ui.p50, painted.rest.p50), (2 * MS, 5 * MS / 2));
    let line = summary.to_string();
    assert!(
        line.contains(
            "; render passes a frame p50 4 max 5, their attachments loaded p50 2.0 max 3.0 MiB, \
             stored p50 4.0 max 6.0 MiB; a one-chain"
        ),
        "{line}"
    );
    assert!(
        line.contains(
            "; a one-chain frame's drawing p50 track 0.10, ops 0.20, plan 0.30, encode 1.00, \
             writes 0.40, submit 0.50 ms in 2 frames; beside the drawing and its waits p50 UI \
             pass 2.0, the rest 2.5 ms; slowest"
        ),
        "{line}"
    );
}

#[test]
fn fingers_hover_and_frames_without_pen_events_are_not_counted() {
    let mut latency = PenLatency::default();
    latency.input(&batch(MotionPhase::HoverMove, ToolKind::Stylus), 0);
    latency.input(&batch(MotionPhase::Down, ToolKind::Finger), 0);
    latency.input(&batch(MotionPhase::Up, ToolKind::Finger), 0);
    latency.skipped();
    assert_eq!(latency.frame((5 * MS, 6 * MS), FrameCost::default()), None);
    latency.input(&batch(MotionPhase::Down, ToolKind::Stylus), 10 * MS);
    latency.frame((12 * MS, 13 * MS), FrameCost::default());
    latency.frame((20 * MS, 21 * MS), FrameCost::default());
    latency.input(&batch(MotionPhase::Up, ToolKind::Stylus), 30 * MS);
    let summary = latency
        .frame((31 * MS, 32 * MS), FrameCost::default())
        .expect("summary");
    assert_eq!((summary.events, summary.frames, summary.skipped), (2, 2, 0));
}

#[test]
fn the_log_line_reads_in_milliseconds() {
    let spread = Spread {
        p50: 8_240_000,
        p95: 12 * MS,
        max: 15_049_999,
    };
    assert_eq!(spread.to_string(), "p50 8.2 p95 12.0 max 15.0 ms");
}
