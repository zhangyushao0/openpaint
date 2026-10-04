//! GPU tests of the walk-through through the whole frame loop: the pen draws under its tip at
//! fit and at any zoom and rotation the fingers give the view; a finger draws nothing (it moves
//! the canvas); a two-finger tap undoes; the side bar looks the same while the pen draws; the
//! screen shows what a cold evaluation of the committed artwork gives, through the view.

mod gpu;
mod screen_map;
mod support;

use openpaint_render::RgbaImage;
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use screen_map::{mapping, pixel_at};
use support::Nowhere;

const SIZE: [u32; 2] = [640, 400];
const PIXELS_PER_POINT: f32 = 2.0;
/// The side bar over the canvas's left edge (8 + 64 points) at density 2, and some room.
const SIDE_BAR_COLUMNS: u32 = 160;
const INK: [u8; 4] = [0, 0, 0, 255];
const PAPER: [u8; 4] = [255; 4];

fn sample(position: [f32; 2], time_ms: i64) -> PointerSample {
    PointerSample {
        position,
        pressure: 0.8,
        tilt: 0.4,
        orientation: 0.3,
        time_ms,
    }
}

fn batch(phase: MotionPhase, pointers: Vec<Pointer>) -> MotionBatch {
    MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers,
    }
}

/// Fingers (or the pen) moving from `from` to `to` in `steps` frames, as Android reports them:
/// down one by one, every pointer in each move, up one by one with the last move.
fn touch(runtime: &mut Runtime, tool: ToolKind, from: &[[f32; 2]], to: &[[f32; 2]], steps: u8) {
    let at = |step: u8, time_ms: i64| -> Vec<Pointer> {
        let t = f32::from(step) / f32::from(steps);
        (0..)
            .zip(from.iter().zip(to))
            .map(|(id, (a, b))| Pointer {
                id,
                tool,
                samples: vec![sample(
                    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
                    time_ms,
                )],
            })
            .collect()
    };
    let count = from.len();
    let mut batches = Vec::new();
    for index in 0..count {
        let phase = if index == 0 {
            MotionPhase::Down
        } else {
            MotionPhase::PointerDown { index }
        };
        let mut pointers = at(0, 0);
        pointers.truncate(index + 1);
        batches.push(batch(phase, pointers));
    }
    for step in 1..steps {
        batches.push(batch(MotionPhase::Move, at(step, i64::from(step) * 16)));
    }
    let end = i64::from(steps) * 16;
    for index in (0..count).rev() {
        let phase = if index == 0 {
            MotionPhase::Up
        } else {
            MotionPhase::PointerUp { index }
        };
        let mut pointers = at(steps, end);
        pointers.truncate(index + 1);
        batches.push(batch(phase, pointers));
    }
    for batch in batches {
        runtime.motion(&batch);
        runtime.frame_offscreen(SIZE).expect("frame");
    }
}

fn swipe(runtime: &mut Runtime, tool: ToolKind, from: [f32; 2], to: [f32; 2]) {
    touch(runtime, tool, &[from], &[to], 20);
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

fn canvas(runtime: &Runtime) -> RgbaImage {
    runtime.read_canvas().expect("readback").expect("canvas")
}

/// Panics unless the screen point `at` and the canvas pixel the view puts under it are ink.
fn ink_under(runtime: &mut Runtime, at: [f32; 2]) {
    let screen = runtime.screenshot(SIZE).expect("screenshot");
    assert_eq!(pixel_at(&screen, at), INK, "ink on screen under {at:?}");
    let point = mapping(runtime, SIZE, PIXELS_PER_POINT).to_canvas(at);
    assert_eq!(
        pixel_at(&canvas(runtime), point),
        INK,
        "ink on the canvas at {point:?}"
    );
}

#[test]
fn the_pen_draws_under_its_tip_and_a_finger_moves_the_canvas_instead() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    swipe(
        &mut runtime,
        ToolKind::Stylus,
        [180.0, 200.0],
        [580.0, 200.0],
    );
    ink_under(&mut runtime, [300.0, 200.0]);
    let screen = runtime.screenshot(SIZE).expect("screenshot");
    assert_eq!(
        pixel_at(&screen, [300.0, 216.0]),
        PAPER,
        "the brush is 12 px wide at 92 %"
    );
    let inked = mapping(&runtime, SIZE, PIXELS_PER_POINT).to_canvas([300.0, 200.0]);
    swipe(
        &mut runtime,
        ToolKind::Finger,
        [300.0, 320.0],
        [420.0, 290.0],
    );
    assert_eq!(strokes(&runtime), 1, "the finger left no ink");
    let pan = runtime.projection().view().pan();
    assert!(
        pan[0] > 20.0 && pan[1] < 0.0,
        "the finger moved the canvas: {pan:?}"
    );
    // The stroke shows where the view now puts it, once the view status has faded (a second
    // by the input clock, which runs on with the wall clock).
    let moved = mapping(&runtime, SIZE, PIXELS_PER_POINT).to_screen(inked);
    assert!(moved[0] > 340.0, "{moved:?}");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    ink_under(&mut runtime, moved);
}

#[test]
fn painting_lands_under_the_pen_at_any_zoom_and_rotation() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    // Two fingers spread and twist about the canvas's middle.
    let from = [[340.0, 250.0], [440.0, 250.0]];
    let to = [[300.0, 180.0], [480.0, 320.0]];
    touch(&mut runtime, ToolKind::Finger, &from, &to, 24);
    let view = runtime.projection().view();
    assert!(view.zoom() > 2.0, "zoomed in: {view:?}");
    assert!(view.rotation() > 0.3, "turned: {view:?}");
    assert_eq!(
        strokes(&runtime),
        0,
        "the fingers drew nothing and undid nothing"
    );
    swipe(
        &mut runtime,
        ToolKind::Stylus,
        [250.0, 150.0],
        [550.0, 330.0],
    );
    assert_eq!(strokes(&runtime), 1);
    for at in [[300.0, 180.0], [400.0, 240.0], [500.0, 300.0]] {
        ink_under(&mut runtime, at);
    }
    // A quick inward pinch fits the canvas again.
    touch(
        &mut runtime,
        ToolKind::Finger,
        &[[200.0, 250.0], [500.0, 250.0]],
        &[[330.0, 250.0], [370.0, 250.0]],
        6,
    );
    assert_eq!(runtime.projection().view(), openpaint_app::CanvasView::FIT);
}

#[test]
fn a_two_finger_tap_undoes_and_a_three_finger_tap_redoes() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    swipe(
        &mut runtime,
        ToolKind::Stylus,
        [180.0, 200.0],
        [580.0, 200.0],
    );
    let two = [[300.0, 300.0], [400.0, 300.0]];
    touch(&mut runtime, ToolKind::Finger, &two, &two, 2);
    assert_eq!(strokes(&runtime), 0, "undone");
    let three = [[300.0, 300.0], [350.0, 300.0], [400.0, 300.0]];
    touch(&mut runtime, ToolKind::Finger, &three, &three, 2);
    assert_eq!(strokes(&runtime), 1, "redone");
}

/// The side bar's pixels on `screen`: every column left of the canvas's free part.
fn side_bar(screen: &RgbaImage) -> Vec<u8> {
    let width = screen.size()[0] as usize;
    let columns = SIDE_BAR_COLUMNS as usize * 4;
    screen
        .pixels()
        .chunks(width * 4)
        .flat_map(|row| row[..columns].to_vec())
        .collect()
}

#[test]
fn the_side_bar_looks_the_same_between_pen_down_and_pen_up() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    for drawn_before in [false, true] {
        if drawn_before {
            swipe(
                &mut runtime,
                ToolKind::Stylus,
                [200.0, 150.0],
                [560.0, 150.0],
            );
        }
        let resting = side_bar(&runtime.screenshot(SIZE).expect("screenshot"));
        let pen = |x| Pointer {
            id: 0,
            tool: ToolKind::Stylus,
            samples: vec![sample([x, 300.0], 0)],
        };
        runtime.motion(&batch(MotionPhase::Down, vec![pen(240.0)]));
        runtime.motion(&batch(MotionPhase::Move, vec![pen(300.0)]));
        let drawing = side_bar(&runtime.screenshot(SIZE).expect("screenshot"));
        assert!(runtime.projection().drawing(), "the pen is on the glass");
        assert!(
            resting == drawing,
            "undo and redo kept their look (step before: {drawn_before})"
        );
        runtime.motion(&batch(MotionPhase::Up, vec![pen(360.0)]));
        runtime.frame_offscreen(SIZE).expect("frame");
    }
}

/// Screen pixels neither ink nor paper along row `y` from `x0` to `x1`, and how many ink runs.
fn partial_pixels(screen: &RgbaImage, y: u32, (x0, x1): (u32, u32)) -> (usize, usize) {
    let red = |x: u32| screen.pixels()[((y * screen.size()[0] + x) * 4) as usize];
    let partial = (x0..x1).filter(|x| !matches!(red(*x), 0 | 255)).count();
    let runs = (x0 + 1..x1)
        .filter(|x| red(*x) < 128 && red(*x - 1) >= 128)
        .count();
    (partial, runs)
}

#[test]
fn zoomed_in_the_screen_shows_ink_edges_one_screen_pixel_wide() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    swipe(
        &mut runtime,
        ToolKind::Stylus,
        [300.0, 120.0],
        [330.0, 300.0],
    );
    let row = (200, (250, 400));
    let fit = partial_pixels(&runtime.screenshot(SIZE).expect("screenshot"), row.0, row.1);
    assert_eq!(fit.1, 1, "one line crosses the row at fit");
    // Spread to about four screen pixels per canvas pixel; the view tier fills in.
    let from = [[270.0, 200.0], [370.0, 200.0]];
    let to = [[95.0, 200.0], [545.0, 200.0]];
    touch(&mut runtime, ToolKind::Finger, &from, &to, 24);
    // Frames 16 ms apart on a held clock, as the host paces them, until it rests.
    for frame in 0..300 {
        if !runtime.moving() {
            break;
        }
        runtime.fix_clock(60_000 + frame * 16);
        runtime.frame_offscreen(SIZE).expect("frame");
    }
    assert!(!runtime.moving(), "the tier filled in");
    let zoom = runtime.projection().view().zoom();
    assert!(zoom > 3.5, "zoomed in: {zoom}");
    let screen = runtime.screenshot(SIZE).expect("screenshot");
    let (partial, runs) = partial_pixels(&screen, 200, (SIDE_BAR_COLUMNS, SIZE[0]));
    assert_eq!(runs, 1, "the line still crosses the row once");
    // The canvas enlarged about four times blurs the edges over 18 pixels (measured).
    assert!(
        partial <= 4,
        "{partial} partly covered pixels across both edges"
    );
}
