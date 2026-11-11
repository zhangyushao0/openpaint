//! The view's own tests: fit, round trips, gestures about their pivot, the zoom's limits, glides.

use super::*;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn canvas(width: u32, height: u32) -> CanvasSize {
    CanvasSize::new(width, height).expect("size")
}

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
}

#[test]
fn fit_shows_the_whole_canvas_centred_with_a_margin() {
    let mapping = CanvasView::FIT.mapping(canvas(2480, 3508), &VIEWPORT);
    let scale = 1716.0 / 3508.0 * FIT_MARGIN;
    assert!((mapping.scale() - scale).abs() < 1e-6);
    assert!(close(mapping.to_screen([1240.0, 1754.0]), [1440.0, 942.0]));
    let top = mapping.to_screen([1240.0, 0.0])[1];
    assert!((top - (942.0 - 1754.0 * scale)).abs() < 0.01);
}

#[test]
fn screen_and_canvas_points_round_trip_at_any_view() {
    let size = canvas(2048, 2048);
    let views = [
        CanvasView::FIT,
        CanvasView::new([120.0, -40.0], 3.5, 0.7).expect("view"),
        CanvasView::new([-900.0, 300.0], 0.3, -2.9).expect("view"),
    ];
    for view in views {
        let mapping = view.mapping(size, &VIEWPORT);
        for point in [[0.0, 0.0], [17.5, 1900.25], [2047.0, 3.0]] {
            let back = mapping.to_canvas(mapping.to_screen(point));
            assert!(
                close(back, point),
                "{view:?}: {point:?} came back as {back:?}"
            );
            let [a, b, c, d, e, f] = mapping.to_canvas_rows();
            let [x, y] = mapping.to_screen(point);
            assert!(close([a * x + b * y + c, d * x + e * y + f], point));
        }
    }
}

#[test]
fn a_gesture_keeps_the_canvas_point_under_its_pivot() {
    let size = canvas(2880, 1716);
    let pivot = [1700.0, 600.0];
    let gesture = ViewGesture {
        pivot,
        translation: [30.0, -12.0],
        zoom: 1.8,
        rotation: 0.4,
    };
    let before = CanvasView::FIT.mapping(size, &VIEWPORT).to_canvas(pivot);
    let view = CanvasView::FIT.after(&gesture, size, &VIEWPORT);
    assert!((view.zoom() - 1.8).abs() < 1e-6);
    assert!((view.rotation() - 0.4).abs() < 1e-6);
    let after = view.mapping(size, &VIEWPORT).to_screen(before);
    assert!(close(after, [1730.0, 588.0]), "{after:?}");
}

#[test]
fn zoom_stops_at_its_limits_and_rotation_wraps() {
    let size = canvas(100, 100);
    let pinch = |zoom, rotation| ViewGesture {
        pivot: [1440.0, 942.0],
        translation: [0.0; 2],
        zoom,
        rotation,
    };
    let big = CanvasView::FIT.after(&pinch(100.0, 3.0), size, &VIEWPORT);
    assert_eq!(big.zoom(), MAX_ZOOM);
    let turned = big.after(&pinch(1.0, 1.0), size, &VIEWPORT);
    assert!((turned.rotation() - (4.0 - TAU)).abs() < 1e-5);
    let small = CanvasView::FIT.after(&pinch(0.001, 0.0), size, &VIEWPORT);
    assert_eq!(small.zoom(), MIN_ZOOM);
    let broken = CanvasView::FIT.after(&pinch(f32::NAN, 0.0), size, &VIEWPORT);
    assert_eq!(broken, CanvasView::FIT);
}

#[test]
fn a_placed_canvas_settles_into_its_view_by_the_short_way() {
    let size = canvas(400, 300);
    let card = ViewMapping::placed(size, [100.0, 50.0], [200.0, 200.0]);
    assert!(close(card.to_screen([0.0, 0.0]), [100.0, 75.0]));
    assert!(close(card.to_screen([400.0, 300.0]), [300.0, 225.0]));
    let turned = CanvasView::new([0.0; 2], 2.0, 3.0).expect("view");
    let to = turned.mapping(size, &VIEWPORT);
    assert_eq!(card.toward(&to, 0.0), card);
    let end = card.toward(&to, 1.0);
    assert!(close(
        end.to_screen([10.0, 20.0]),
        to.to_screen([10.0, 20.0])
    ));
    let half = card.toward(&to, 0.5);
    assert!((half.scale() - (card.scale() * to.scale()).sqrt()).abs() < 1e-4);
    assert!(
        (half.sin.atan2(half.cos) - 1.5).abs() < 1e-4,
        "never the long way"
    );
}
