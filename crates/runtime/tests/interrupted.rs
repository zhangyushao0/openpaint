//! S50: the app interrupted mid-stroke through the whole frame loop. The window goes (the screen
//! turned off, the app sent home) while the pen is down: the stroke drawn so far is committed with
//! every sample, as its lift would, the canvas then equals a cold evaluation of the artwork, the
//! view (zoom, pan, rotation) is as it was, the pen's cancel that follows changes nothing, and
//! the committed stroke is stored. Leaving the app mid-stroke (`onPause`) stores it too, before
//! the process may be killed.

mod gpu;
mod hand;

use std::path::PathBuf;

use hand::{Hand, assert_cold};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use openpaint_store::Library;

const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// Right of the side bar.
fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=20u8)
        .map(|i| [300.0 + 40.0 * f32::from(i), y])
        .collect()
}

fn library_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("runtime-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear an old library");
    }
    dir
}

fn strokes(runtime: &Runtime) -> Vec<usize> {
    let artwork = runtime.artwork().expect("artwork");
    let layer = &artwork.layers()[0];
    layer.strokes().iter().map(|s| s.samples().len()).collect()
}

/// One pen event of `phase` at `at`, then the frames the runtime asks for.
fn pen(runtime: &mut Runtime, hand: &mut Hand, phase: MotionPhase, at: [f32; 2]) {
    hand.clock += 8;
    runtime.fix_clock(hand.clock);
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
                time_ms: hand.clock,
            }],
        }],
    });
    settle(runtime, hand);
}

/// Frames on a held clock until nothing moves, fills in or waits for the store.
fn settle(runtime: &mut Runtime, hand: &mut Hand) {
    for _ in 0..400 {
        hand.clock += 16;
        runtime.fix_clock(hand.clock);
        let again = runtime.frame_offscreen(SIZE).expect("frame");
        if !again && !runtime.moving() && !runtime.storing() {
            return;
        }
    }
}

/// Two fingers moving apart and turning: the view zooms, pans and rotates.
fn pinch(runtime: &mut Runtime, hand: &mut Hand) {
    let (from, to) = (
        [[700.0, 500.0], [900.0, 500.0]],
        [[620.0, 420.0], [960.0, 600.0]],
    );
    let at = |step: u8, clock: i64| -> Vec<Pointer> {
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
                    time_ms: clock,
                }],
            })
            .collect()
    };
    let mut batches = vec![
        (MotionPhase::Down, at(0, hand.clock)[..1].to_vec()),
        (MotionPhase::PointerDown { index: 1 }, at(0, hand.clock)),
    ];
    batches.extend((1..=12).map(|step| {
        (
            MotionPhase::Move,
            at(step, hand.clock + 16 * i64::from(step)),
        )
    }));
    batches.push((
        MotionPhase::PointerUp { index: 1 },
        at(12, hand.clock + 208),
    ));
    batches.push((MotionPhase::Up, at(12, hand.clock + 208)[..1].to_vec()));
    for (phase, pointers) in batches {
        hand.clock += 16;
        runtime.fix_clock(hand.clock);
        runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers,
        });
        runtime.frame_offscreen(SIZE).expect("frame");
    }
    settle(runtime, hand);
}

/// The app on the library in `dir`, its canvas open.
fn start(dir: &PathBuf) -> Runtime {
    let store = Box::new(Library::at(dir));
    let mut runtime =
        Runtime::start_on(gpu::core(), PIXELS_PER_POINT, store).expect("the runtime starts");
    let started = std::time::Instant::now();
    while runtime.frame_offscreen(SIZE).expect("frame") || runtime.artwork().is_none() {
        assert!(started.elapsed().as_secs() < 10, "the library did not open");
    }
    runtime
}

#[test]
fn a_stroke_being_drawn_when_the_window_goes_is_committed_and_the_view_kept() {
    let dir = library_dir("detached");
    let mut runtime = start(&dir);
    let mut hand = Hand::new(SIZE);
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
    assert_eq!(strokes(&runtime).len(), 1, "drawn, not committed yet");
    runtime.detach();
    assert!(
        runtime.drawn().expect("a canvas").1.is_none(),
        "no stroke being drawn"
    );
    assert_eq!(
        strokes(&runtime)[1],
        10,
        "committed with every sample so far"
    );
    // The window comes back (offscreen: frames go on).
    settle(&mut runtime, &mut hand);
    assert_eq!(runtime.projection().view(), view, "the view as it was");
    let core = runtime.core();
    assert_cold(&core, &runtime, "after the window came back");
    let shown = runtime.read_canvas().expect("read").expect("canvas");
    // Android cancels the pen's gesture once the window went: nothing changes.
    pen(&mut runtime, &mut hand, MotionPhase::Cancel, points[9]);
    assert_eq!(strokes(&runtime), [21, 10]);
    let after = runtime.read_canvas().expect("read").expect("canvas");
    assert_eq!(after.content_hash(), shown.content_hash());
    // One step: undo takes it back, redo brings it again.
    hand.tap(&mut runtime, "Undo");
    assert_eq!(strokes(&runtime), [21]);
    hand.tap(&mut runtime, "Redo");
    assert_eq!(strokes(&runtime), [21, 10]);
    settle(&mut runtime, &mut hand);
    assert_cold(&core, &runtime, "after undo and redo");
    let parts = runtime.artwork().expect("artwork").to_parts();
    drop(runtime);
    let reopened = start(&dir);
    assert_eq!(
        reopened.artwork().expect("artwork").to_parts(),
        parts,
        "stored"
    );
}

#[test]
fn leaving_the_app_mid_stroke_stores_the_stroke_drawn_so_far() {
    let dir = library_dir("left");
    let mut runtime = start(&dir);
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(300.0));
    let points = line(480.0);
    pen(&mut runtime, &mut hand, MotionPhase::Down, points[0]);
    for at in &points[1..15] {
        pen(&mut runtime, &mut hand, MotionPhase::Move, *at);
    }
    // `onPause`, and the system ends the process at once.
    runtime.leave();
    let parts = runtime.artwork().expect("artwork").to_parts();
    assert_eq!(strokes(&runtime), [21, 15]);
    drop(runtime);
    let reopened = start(&dir);
    assert_eq!(reopened.artwork().expect("artwork").to_parts(), parts);
    assert_cold(&reopened.core(), &reopened, "reopened");
}
