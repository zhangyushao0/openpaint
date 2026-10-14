//! Frame pacing (S56): while the GPU is behind, frame calls are skipped at once and input waits
//! for the next frame drawn, which takes every sample since. A stroke drawn while nearly every
//! frame is skipped (the GPU held behind, as a backlog holds it) commits every sample and shows
//! as a cold evaluation does, as the same stroke drawn without skipping.

mod gpu;
mod hand;
mod support;

use hand::{Hand, assert_cold};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use support::Nowhere;

const SIZE: [u32; 2] = [1280, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// Right of the side bar, a wave of 31 points.
fn wave(y: f32) -> Vec<[f32; 2]> {
    (0..=30u8)
        .map(|i| {
            let t = f32::from(i) / 30.0;
            [200.0 + 800.0 * t, y + 120.0 * (6.0 * t).sin()]
        })
        .collect()
}

/// One pen event at `at` and one frame call; returns whether that call was skipped.
fn pen(runtime: &mut Runtime, hand: &mut Hand, phase: MotionPhase, at: [f32; 2]) -> bool {
    hand.clock += 8;
    let wanted = runtime.motion(&MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id: 0,
            tool: ToolKind::Stylus,
            samples: vec![PointerSample {
                position: at,
                pressure: 0.8,
                tilt: 0.0,
                orientation: 0.0,
                time_ms: hand.clock,
            }],
        }],
    });
    assert!(wanted, "a pen event asks for a frame");
    assert!(runtime.frame_offscreen(SIZE).expect("frame"));
    runtime.paced().skipped
}

/// Frames until the runtime rests.
fn rest(runtime: &mut Runtime) {
    for _ in 0..240 {
        if !runtime.frame_offscreen(SIZE).expect("frame") {
            return;
        }
    }
}

/// The samples of the last stroke on the bottom layer.
fn last_samples(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    let last = artwork.layers()[0].strokes().last().expect("a stroke");
    last.samples().len()
}

#[test]
fn a_stroke_drawn_while_frames_are_skipped_commits_every_sample() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    // Without skipping: the reference.
    hand.gesture(&mut runtime, ToolKind::Stylus, &wave(300.0));
    let drawn = last_samples(&runtime);
    rest(&mut runtime);
    // The pen touches down in a frame drawn; then the GPU is held behind: every frame call while
    // the pen moves and lifts is skipped, and the input waits.
    let points = wave(560.0);
    assert!(!pen(&mut runtime, &mut hand, MotionPhase::Down, points[0]));
    runtime.hold_gpu(u32::MAX);
    let last = points.len() - 1;
    for at in &points[1..last] {
        assert!(pen(&mut runtime, &mut hand, MotionPhase::Move, *at));
    }
    assert!(pen(&mut runtime, &mut hand, MotionPhase::Up, points[last]));
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(artwork.layers()[0].strokes().len(), 1, "nothing drawn yet");
    // The GPU catches up: the next frame drawn takes every sample and the lift.
    runtime.hold_gpu(0);
    rest(&mut runtime);
    assert!(!runtime.paced().skipped);
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(
        artwork.layers()[0].strokes().len(),
        2,
        "the stroke committed"
    );
    assert_eq!(
        last_samples(&runtime),
        drawn,
        "every sample, as the stroke drawn without skipping"
    );
    assert_cold(&runtime.core(), &mut runtime, "after the skipped frames");
    // One step: undo takes it back whole.
    hand.tap(&mut runtime, "Undo");
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(artwork.layers()[0].strokes().len(), 1, "undone");
    assert_cold(&runtime.core(), &mut runtime, "after undo");
}
