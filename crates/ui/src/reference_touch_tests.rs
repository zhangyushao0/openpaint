use egui::Vec2;
use openpaint_app::{InputSample, ReferenceImage};
use openpaint_core::Color;

use super::*;
use crate::components::{FloatingLayout, MAT};

/// A window at (100, 100), 360 x 200 points, on a 1000 x 800 screen at density 1, showing a
/// 4 x 2 image: red on the left half, blue on the right.
fn window() -> ReferenceWindow {
    let ctx = egui::Context::default();
    let pixels: Vec<u8> = (0..8)
        .flat_map(|i| {
            if i % 4 < 2 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        })
        .collect();
    let image = ReferenceImage::fitted([4, 2], &pixels).expect("image");
    let mut window = ReferenceWindow::default();
    let _texture = window.load(&ctx, &image);
    window.pixels_per_point = 1.0;
    let place = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(360.0, 200.0));
    window.place = Some(place);
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
    let layout = FloatingLayout {
        window: place,
        picture: place.shrink(MAT),
        bar: None,
        grip: None,
        resize: None,
    };
    window.layout = Some((layout, screen));
    window
}

/// The bar shown: a 48-point bar whose right 48 points hold ×, a resize mark at the corner.
fn with_bar(window: &mut ReferenceWindow) {
    let (mut layout, screen) = window.layout.expect("layout");
    let bar = Rect::from_min_size(layout.window.min, Vec2::new(layout.window.width(), 48.0));
    layout.bar = Some(bar);
    layout.grip = Some(Rect::from_min_max(
        bar.min,
        Pos2::new(bar.right() - 48.0, bar.bottom()),
    ));
    let corner = layout.window.max - Vec2::splat(48.0);
    layout.resize = Some(Rect::from_min_max(corner, layout.window.max));
    window.layout = Some((layout, screen));
    window.bar_until = Some(BAR_IDLE_MS);
}

fn batch(id: i32, device: Device, phase: ContactPhase, [x, y]: [f32; 2]) -> InputBatch {
    let sample = InputSample {
        position: [x, y],
        pressure: 1.0,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms: 0,
    };
    InputBatch {
        contacts: vec![Contact {
            id,
            device,
            phase,
            samples: vec![sample],
        }],
    }
}

fn finger(phase: ContactPhase, at: [f32; 2]) -> InputBatch {
    batch(1, Device::Finger, phase, at)
}

#[test]
fn a_tap_shows_the_bar_and_a_touch_outside_or_a_second_tap_hides_it() {
    let mut window = window();
    window.route(&[finger(ContactPhase::Down, [200.0, 200.0])], 10, false);
    assert!(window.bar_until.is_none(), "not on the touch down");
    window.route(&[finger(ContactPhase::Up, [203.0, 200.0])], 20, false);
    assert_eq!(window.bar_until, Some(20 + BAR_IDLE_MS));
    window.route(&[finger(ContactPhase::Down, [800.0, 600.0])], 30, false);
    assert!(window.bar_until.is_none(), "a touch outside hides it");
    let pen = |phase| batch(2, Device::Pen, phase, [200.0, 200.0]);
    window.route(&[pen(ContactPhase::Down)], 40, false);
    window.route(&[pen(ContactPhase::Up)], 50, false);
    assert!(window.bar_until.is_some(), "the pen taps too");
    window.route(&[finger(ContactPhase::Down, [200.0, 200.0])], 60, false);
    window.route(&[finger(ContactPhase::Up, [200.0, 200.0])], 70, false);
    assert!(window.bar_until.is_none(), "a second tap hides it");
}

#[test]
fn the_grip_moves_the_whole_window_and_the_corner_resizes_it() {
    let mut window = window();
    with_bar(&mut window);
    window.route(&[finger(ContactPhase::Down, [150.0, 120.0])], 0, false);
    window.route(&[finger(ContactPhase::Move, [100.0, 140.0])], 10, false);
    let moved = window.place.expect("place");
    assert_eq!(
        (moved.min, moved.size()),
        (Pos2::new(50.0, 120.0), Vec2::new(360.0, 200.0))
    );
    assert_eq!(
        window.bar_until,
        Some(10 + BAR_IDLE_MS),
        "use keeps the bar"
    );
    window.route(&[finger(ContactPhase::Up, [100.0, 140.0])], 20, false);
    // The layout follows on the next pass; the corner is still where it was.
    window.route(&[finger(ContactPhase::Down, [450.0, 290.0])], 30, false);
    window.route(&[finger(ContactPhase::Up, [500.0, 330.0])], 40, false);
    let resized = window.place.expect("place");
    assert_eq!(
        (resized.min, resized.size()),
        (moved.min, Vec2::new(410.0, 240.0))
    );
    assert!(window.zoom == 1.0, "neither touched the picture");
}

#[test]
fn a_pinch_zooms_the_picture_about_its_middle_and_one_finger_pans_it() {
    let mut window = window();
    let pair = |phase, spread: f32| InputBatch {
        contacts: [(1, 280.0 - spread), (2, 280.0 + spread)]
            .into_iter()
            .flat_map(|(id, x)| batch(id, Device::Finger, phase, [x, 200.0]).contacts)
            .collect(),
    };
    window.route(&[pair(ContactPhase::Down, 40.0)], 0, false);
    window.route(&[pair(ContactPhase::Move, 80.0)], 10, false);
    window.route(&[pair(ContactPhase::Up, 80.0)], 20, false);
    assert!(window.zoom > 1.5, "zoomed: {}", window.zoom);
    assert!(window.bar_until.is_none(), "a pinch is no tap");
    // About the picture's middle (the pinch's), so the image's middle stays there.
    assert!(
        (window.centre - Vec2::new(2.0, 1.0)).length() < 1e-4,
        "{:?}",
        window.centre
    );
    window.route(&[finger(ContactPhase::Down, [280.0, 200.0])], 30, false);
    window.route(&[finger(ContactPhase::Up, [258.0, 200.0])], 40, false);
    assert!(
        window.centre.x > 2.0,
        "dragged left, it shows more of the right"
    );
}

#[test]
fn picking_previews_the_image_under_the_contact_and_lifting_picks_the_last_preview() {
    let mut window = window();
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    let blue = Color::rgb(0.0, 0.0, 1.0).expect("blue");
    let down = window.route(&[finger(ContactPhase::Down, [150.0, 200.0])], 0, true);
    assert_eq!(down, [Intent::Tool(ToolIntent::Preview(red))]);
    let right = window.route(&[finger(ContactPhase::Move, [400.0, 200.0])], 10, true);
    assert_eq!(right, [Intent::Tool(ToolIntent::Preview(blue))]);
    let off = window.route(&[finger(ContactPhase::Move, [600.0, 200.0])], 20, true);
    assert!(off.is_empty(), "off the picture: the last preview stays");
    assert_eq!(
        window.picking_at(),
        Some([600.0, 200.0]),
        "the loupe follows"
    );
    let up = window.route(&[finger(ContactPhase::Up, [610.0, 200.0])], 30, true);
    assert_eq!(up, [Intent::Tool(ToolIntent::Pick)]);
    assert_eq!(window.picking_at(), None);
    assert!(window.bar_until.is_none(), "picking is no tap");
}
