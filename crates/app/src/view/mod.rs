//! The canvas view (Procreate's model, as draweave tuned it): how the open artwork shows in the
//! canvas area. Zoom is relative to fit, so the whole canvas (or the rectangle of it the view
//! fits, `Fitted`) shows with a margin at zoom 1 on any screen and after any rotation of the
//! device; pan is in points from the area's centre; rotation is free. The view belongs to the
//! open session: never artwork content, never a history step, not stored (every artwork opens
//! at fit). `ViewMapping` turns it into the one screen-to-canvas map that input, rendering and
//! picking all use for a frame.

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
    /// The rectangle fit is of; `None` is the whole canvas.
    fits: Option<Fitted>,
}

/// A rectangle of the canvas, in canvas pixels, that a view fits instead of the whole canvas:
/// fit and the zoom's limits are of it, and the view centres on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fitted {
    pub origin: [u32; 2],
    pub size: [u32; 2],
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
    /// The rectangle the view fits (the whole canvas unless it names one), upright and centred
    /// (a quick pinch).
    Fit,
}

impl CanvasView {
    pub const FIT: Self = Self {
        pan: [0.0; 2],
        zoom: 1.0,
        rotation: 0.0,
        fits: None,
    };

    /// Fit of `fits` (the whole canvas when `None`): centred on it, upright.
    #[must_use]
    pub const fn fitting(fits: Option<Fitted>) -> Self {
        Self { fits, ..Self::FIT }
    }

    /// A view of these values fitting the whole canvas; `None` unless every value is finite,
    /// the zoom in `MIN_ZOOM..=MAX_ZOOM` and the pan within reach.
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
            fits: None,
        })
    }

    /// This view fitting `fits` instead (the whole canvas when `None`), its pan, zoom and
    /// rotation kept.
    #[must_use]
    pub const fn within(self, fits: Option<Fitted>) -> Self {
        Self { fits, ..self }
    }

    /// The rectangle fit is of; `None` is the whole canvas.
    #[must_use]
    pub const fn fits(self) -> Option<Fitted> {
        self.fits
    }

    /// Points from the canvas area's centre to the centre of what the view fits.
    #[must_use]
    pub const fn pan(self) -> [f32; 2] {
        self.pan
    }

    /// Relative to fit: 1 shows what the view fits with the `FIT_MARGIN`.
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
        let whole = Fitted {
            origin: [0, 0],
            size: [canvas.width(), canvas.height()],
        };
        let Fitted { origin, size } = self.fits.unwrap_or(whole);
        let [x, y] = origin.map(|at| at as f32);
        let [w, h] = size.map(|side| side.max(1) as f32);
        let [vw, vh] = viewport.size.map(|side| side.max(1.0));
        let fit = (vw / w).min(vh / h) * FIT_MARGIN;
        let (sin, cos) = self.rotation.sin_cos();
        let ppp = viewport.pixels_per_point;
        ViewMapping {
            canvas_centre: [x + w / 2.0, y + h / 2.0],
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

    /// The whole of `canvas`, upright, fitted into the screen rectangle at `origin` of `size`
    /// pixels and centred there: where a gallery card shows it.
    #[must_use]
    pub fn placed(canvas: CanvasSize, origin: [f32; 2], size: [f32; 2]) -> Self {
        let [cw, ch] = [canvas.width() as f32, canvas.height() as f32];
        Self {
            canvas_centre: [cw / 2.0, ch / 2.0],
            screen_centre: [origin[0] + size[0] / 2.0, origin[1] + size[1] / 2.0],
            scale: (size[0] / cw).min(size[1] / ch).max(f32::MIN_POSITIVE),
            sin: 0.0,
            cos: 1.0,
        }
    }

    /// The way from this map to `to` (of the same canvas) at `t`, 0 this one and 1 `to`: the
    /// canvas point `to` centres on moves in a line from where this map shows it, the scale
    /// changes by an even ratio and the rotation turns the short way round. For showing a view
    /// settle; input always uses `to`.
    #[must_use]
    pub fn toward(&self, to: &Self, t: f32) -> Self {
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        let from = self.sin.atan2(self.cos);
        let angle = from + wrapped(to.sin.atan2(to.cos) - from) * t;
        let (sin, cos) = angle.sin_cos();
        let start = self.to_screen(to.canvas_centre);
        Self {
            canvas_centre: to.canvas_centre,
            screen_centre: [
                lerp(start[0], to.screen_centre[0]),
                lerp(start[1], to.screen_centre[1]),
            ],
            scale: self.scale * (to.scale / self.scale).powf(t),
            sin,
            cos,
        }
    }
}

/// `angle` in `-pi..=pi`.
fn wrapped(angle: f32) -> f32 {
    let turned = (angle + PI).rem_euclid(TAU) - PI;
    if turned.is_finite() { turned } else { 0.0 }
}

#[cfg(test)]
mod tests;
