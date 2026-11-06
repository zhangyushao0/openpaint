//! S50: the app interrupted mid-stroke through the whole frame loop. The window goes (the screen
//! turned off, the app sent home) while the pen is down: the stroke drawn so far is committed with
//! every sample, as its lift would, the canvas then equals a cold evaluation of the artwork, the
//! view (zoom, pan, rotation) is as it was, the pen's cancel that follows changes nothing, and
//! the committed stroke is stored. Leaving the app mid-stroke (`onPause`) stores it too, before
//! the process may be killed.

use std::time::Instant;

use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use openpaint_store::Library;

use crate::support::hand::HANG;
use crate::support::{Hand, TestDir, assert_cold, frame, line, settle, start};

const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// The samples of each stroke on the bottom layer.
fn samples(runtime: &Runtime) -> Vec<usize> {
    let artwork = runtime.artwork().expect("artwork");
    let layer = &artwork.layers()[0];
    layer.strokes().iter().map(|s| s.samples().len()).collect()
}

/// One pen event of `phase` at `at`, then one frame 8 ms later, as on the device.
fn pen(runtime: &mut Runtime, hand: &mut Hand, phase: MotionPhase, at: [f32; 2]) {
    let time_ms = hand.tick(runtime, 8);
    runtime.motion(&MotionBatch {
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
                time_ms,
            }],
        }],
    });
    hand.tick(runtime, 8);
    hand.frame_now(runtime);
}

/// Two fingers moving apart and turning: the view zooms, pans and rotates.
fn pinch(runtime: &mut Runtime, hand: &mut Hand) {
    let (from, to) = (
        [[700.0, 500.0], [900.0, 500.0]],
        [[620.0, 420.0], [960.0, 600.0]],
    );
    let at = |step: u8, time_ms: i64| -> Vec<Pointer> {
        let t = f32::from(step) / 12.0;
        (0..)
            .zip(from.iter().zip(&to))
            .map(|(id, (a, b)): (i32, (&[f32; 2], &[f32; 2]))| Pointer {
                id,
                tool: ToolKind::Finger,
                samples: vec![PointerSample {
                    position: [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
                    pressure: 1.0,
                    tilt: 0.0,
                    orientation: 0.0,
                    time_ms,
                }],
            })
            .collect()
    };
    // Each batch's phase, step and fingers, a batch 16 ms after the one before, with a frame.
    let mut steps = vec![
        (MotionPhase::Down, 0, 1),
        (MotionPhase::PointerDown { index: 1 }, 0, 2),
    ];
    steps.extend((1..=12).map(|step| (MotionPhase::Move, step, 2)));
    steps.push((MotionPhase::PointerUp { index: 1 }, 12, 2));
    steps.push((MotionPhase::Up, 12, 1));
    for (phase, step, fingers) in steps {
        let time_ms = hand.tick(runtime, 16);
        runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: at(step, time_ms)[..fingers].to_vec(),
        });
        hand.frame_now(runtime);
    }
    settle(runtime, hand);
}

#[test]
fn a_stroke_being_drawn_when_the_window_goes_is_committed_and_the_view_kept() {
    let dir = TestDir::new("detached");
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(300.0));
    pinch(&mut runtime, &mut hand);
    let view = runtime.projection().view();
    assert_ne!(
        view,
        openpaint_app::CanvasView::FIT,
        "zoomed, panned and turned"
    );
    // The pen is down and has drawn ten samples when the window goes.
    let points = line(480.0);
    pen(&mut runtime, &mut hand, MotionPhase::Down, points[0]);
    for at in &points[1..10] {
        pen(&mut runtime, &mut hand, MotionPhase::Move, *at);
    }
    assert_eq!(samples(&runtime).len(), 1, "drawn, not committed yet");
    runtime.detach();
    assert!(
        runtime.drawn().expect("a canvas").1.is_none(),
        "no stroke being drawn"
    );
    assert_eq!(
        samples(&runtime)[1],
        10,
        "committed with every sample so far"
    );
    // The window comes back (offscreen: frames go on), until nothing moves: the pen's contact,
    // never lifted, wants frames until the platform cancels it.
    let back = Instant::now();
    frame(&mut runtime, &mut hand);
    while runtime.moving() {
        frame(&mut runtime, &mut hand);
        assert!(back.elapsed() < HANG, "still moving");
    }
    assert_eq!(runtime.projection().view(), view, "the view as it was");
    let core = runtime.core();
    assert_cold(&core, &mut runtime, "after the window came back");
    let shown = runtime.read_canvas().expect("read").expect("canvas");
    // Android cancels the pen's gesture once the window went: nothing changes.
    pen(&mut runtime, &mut hand, MotionPhase::Cancel, points[9]);
    assert_eq!(samples(&runtime), [21, 10]);
    let after = runtime.read_canvas().expect("read").expect("canvas");
    assert_eq!(after.content_hash(), shown.content_hash());
    // One step: undo takes it back, redo brings it again.
    hand.tap(&mut runtime, "Undo");
    assert_eq!(samples(&runtime), [21]);
    hand.tap(&mut runtime, "Redo");
    assert_eq!(samples(&runtime), [21, 10]);
    settle(&mut runtime, &mut hand);
    assert_cold(&core, &mut runtime, "after undo and redo");
    let parts = runtime.artwork().expect("artwork").to_parts();
    drop(runtime);
    let (reopened, _) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    assert_eq!(
        reopened.artwork().expect("artwork").to_parts(),
        parts,
        "stored"
    );
}

#[test]
fn leaving_the_app_mid_stroke_stores_the_stroke_drawn_so_far() {
    let dir = TestDir::new("left");
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(300.0));
    let points = line(480.0);
    pen(&mut runtime, &mut hand, MotionPhase::Down, points[0]);
    for at in &points[1..15] {
        pen(&mut runtime, &mut hand, MotionPhase::Move, *at);
    }
    // `onPause`, and the system ends the process at once.
    runtime.leave();
    let parts = runtime.artwork().expect("artwork").to_parts();
    assert_eq!(samples(&runtime), [21, 15]);
    drop(runtime);
    let (mut reopened, _) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    assert_eq!(reopened.artwork().expect("artwork").to_parts(), parts);
    assert_cold(&reopened.core(), &mut reopened, "reopened");
}
