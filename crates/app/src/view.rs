//! The canvas view (Procreate's model, as draweave tuned it): how the open artwork shows in the
//! canvas area. Zoom is relative to fit, so the whole canvas shows with a margin at zoom 1 on
//! any screen and after any rotation of the device; pan is in points from the area's centre;
//! rotation is free. The view belongs to the open session: never artwork content, never a
//! history step, not stored (every artwork opens at fit). `ViewMapping` turns it into the one
//! screen-to-canvas map that input, rendering and picking all use for a frame.

use std::f32::consts::{PI, TAU};

use openpaint_core::CanvasSize;

/// At zoom 1 the canvas fills this share of the canvas area, so a margin of surround shows.
pub const FIT_MARGIN: f32 = 0.92;
/// The view zooms from a quarter of fit to eight times fit.
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 8.0;
/// Pan stays within this many points of the centre, far past any screen.
const PAN_LIMIT: f32 = 1.0e6;

/// How the artwork shows. The default is fit: centred, upright, the whole canvas in view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasView {
    pan: [f32; 2],
    zoom: f32,
    rotation: f32,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self::FIT
    }
}

/// The canvas area on screen for one frame: its top-left corner and size in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub origin: [f32; 2],
    pub size: [f32; 2],
    pub pixels_per_point: f32,
}

/// One frame of a navigation gesture, in screen pixels: the canvas point under `pivot` stays
/// under it while the view zooms by `zoom` and turns by `rotation` (radians, clockwise on
/// screen), then moves by `translation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewGesture {
    pub pivot: [f32; 2],
    pub translation: [f32; 2],
    pub zoom: f32,
    pub rotation: f32,
}

/// What the user does with the view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewIntent {
    /// One frame of navigation over the canvas area `viewport`.
    Gesture {
        gesture: ViewGesture,
        viewport: Viewport,
    },
    /// The whole canvas, upright and centred (a quick pinch).
    Fit,
}

impl CanvasView {
    pub const FIT: Self = Self {
        pan: [0.0; 2],
        zoom: 1.0,
        rotation: 0.0,
    };

    /// A view of these values; `None` unless every value is finite, the zoom in
    /// `MIN_ZOOM..=MAX_ZOOM` and the pan within reach.
    #[must_use]
    pub fn new(pan: [f32; 2], zoom: f32, rotation: f32) -> Option<Self> {
        let finite = pan.iter().chain([&zoom, &rotation]).all(|v| v.is_finite());
        let valid = finite
            && (MIN_ZOOM..=MAX_ZOOM).contains(&zoom)
            && pan.iter().all(|v| v.abs() <= PAN_LIMIT);
        valid.then(|| Self {
            pan,
            zoom,
            rotation: wrapped(rotation),
        })
    }

    /// Points from the canvas area's centre to the canvas's centre.
    #[must_use]
    pub const fn pan(self) -> [f32; 2] {
        self.pan
    }

    /// Relative to fit: 1 shows the whole canvas with the `FIT_MARGIN`.
    #[must_use]
    pub const fn zoom(self) -> f32 {
        self.zoom
    }

    /// Radians in `-pi..=pi`, clockwise on screen.
    #[must_use]
    pub const fn rotation(self) -> f32 {
        self.rotation
    }

    /// The map between screen and canvas pixels of `canvas` shown in `viewport`.
    #[must_use]
    pub fn mapping(self, canvas: CanvasSize, viewport: &Viewport) -> ViewMapping {
        let [cw, ch] = [canvas.width() as f32, canvas.height() as f32];
        let [vw, vh] = viewport.size.map(|side| side.max(1.0));
        let fit = (vw / cw).min(vh / ch) * FIT_MARGIN;
        let (sin, cos) = self.rotation.sin_cos();
        let ppp = viewport.pixels_per_point;
        ViewMapping {
            canvas_centre: [cw / 2.0, ch / 2.0],
            screen_centre: [
                viewport.origin[0] + vw / 2.0 + self.pan[0] * ppp,
                viewport.origin[1] + vh / 2.0 + self.pan[1] * ppp,
            ],
            scale: fit * self.zoom,
            sin,
            cos,
        }
    }

    /// This view after one frame of `gesture` over `canvas` in `viewport`. The zoom stops at its
    /// limits with the pivot's canvas point kept under the pivot; a gesture with values that
    /// are not finite changes nothing.
    #[must_use]
    pub fn after(self, gesture: &ViewGesture, canvas: CanvasSize, viewport: &Viewport) -> Self {
        let ViewGesture {
            pivot,
            translation,
            zoom,
            rotation,
        } = *gesture;
        let values = [
            pivot[0],
            pivot[1],
            translation[0],
            translation[1],
            zoom,
            rotation,
        ];
        if values.iter().any(|v| !v.is_finite()) || zoom <= 0.0 {
            return self;
        }
        let anchor = self.mapping(canvas, viewport).to_canvas(pivot);
        let mut next = Self {
            zoom: (self.zoom * zoom).clamp(MIN_ZOOM, MAX_ZOOM),
            rotation: wrapped(self.rotation + rotation),
            ..self
        };
        let projected = next.mapping(canvas, viewport).to_screen(anchor);
        let ppp = viewport.pixels_per_point.max(f32::MIN_POSITIVE);
        for axis in 0..2 {
            let moved = translation[axis] + pivot[axis] - projected[axis];
            next.pan[axis] = (next.pan[axis] + moved / ppp).clamp(-PAN_LIMIT, PAN_LIMIT);
        }
        next
    }
}

/// The view for one frame as an affine map: screen pixels (as the host reports positions) to
/// canvas pixels and back. Rotation turns about the canvas's centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewMapping {
    canvas_centre: [f32; 2],
    /// Where the canvas's centre shows, in screen pixels.
    screen_centre: [f32; 2],
    /// Screen pixels per canvas pixel.
    scale: f32,
    sin: f32,
    cos: f32,
}

impl ViewMapping {
    /// Screen pixels per canvas pixel.
    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    #[must_use]
    pub fn to_screen(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        let [dx, dy] = [x - self.canvas_centre[0], y - self.canvas_centre[1]];
        [
            self.screen_centre[0] + (dx * self.cos - dy * self.sin) * self.scale,
            self.screen_centre[1] + (dx * self.sin + dy * self.cos) * self.scale,
        ]
    }

    #[must_use]
    pub fn to_canvas(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        let [dx, dy] = [x - self.screen_centre[0], y - self.screen_centre[1]];
        [
            self.canvas_centre[0] + (dx * self.cos + dy * self.sin) / self.scale,
            self.canvas_centre[1] + (dy * self.cos - dx * self.sin) / self.scale,
        ]
    }

    /// `to_canvas` as the rows `[a, b, c, d, e, f]` of `canvas = (a x + b y + c, d x + e y + f)`
    /// for a screen point `(x, y)`, for the renderer.
    #[must_use]
    pub fn to_canvas_rows(&self) -> [f32; 6] {
        let [a, b] = [self.cos / self.scale, self.sin / self.scale];
        let [d, e] = [-self.sin / self.scale, self.cos / self.scale];
        let [sx, sy] = self.screen_centre;
        let [cx, cy] = self.canvas_centre;
        [a, b, cx - a * sx - b * sy, d, e, cy - d * sx - e * sy]
    }
}

/// `angle` in `-pi..=pi`.
fn wrapped(angle: f32) -> f32 {
    let turned = (angle + PI).rem_euclid(TAU) - PI;
    if turned.is_finite() { turned } else { 0.0 }
}

#[cfg(test)]
mod tests {
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
    fn views_are_checked() {
        assert!(CanvasView::new([0.0, 0.0], 8.5, 0.0).is_none());
        assert!(CanvasView::new([f32::INFINITY, 0.0], 1.0, 0.0).is_none());
        assert!(CanvasView::new([0.0, 0.0], 1.0, f32::NAN).is_none());
        assert_eq!(CanvasView::new([0.0; 2], 1.0, 0.0), Some(CanvasView::FIT));
    }
}
