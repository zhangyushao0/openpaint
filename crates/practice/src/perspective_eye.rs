//! The eye boxes and squares in perspective are seen by (`perspective.rs`, `ellipse.rs`): a
//! pinhole at the world's origin (y up, z away), its focal length 1, looking level or turned up
//! or down; a box's corners in the world; and a box's image made the size it shows in the frame
//! and moved there, its horizon and vanishing points with it (a picture made larger and moved
//! is still the same view, seen with a longer lens).

use crate::path::Point;
use crate::perspective::{Horizon, PerspectiveBox};

/// The box's three directions in the world (y up, z away from the eye) turned `yaw` about the
/// vertical, in `Direction`'s order: the left one, the right one, up.
pub(crate) fn directions(yaw: f64) -> [[f64; 3]; 3] {
    let (sin, cos) = yaw.sin_cos();
    [[-sin, 0.0, cos], [cos, 0.0, sin], [0.0, 1.0, 0.0]]
}

/// The corners of the box `[width, height, depth]` round `middle`, turned `yaw`.
pub(crate) fn box_corners(
    middle: [f64; 3],
    yaw: f64,
    [width, height, depth]: [f64; 3],
) -> [[f64; 3]; 8] {
    let [left, right, up] = directions(yaw);
    let step = |d: [f64; 3], by: f64| d.map(|v| v * by);
    let origin: [f64; 3] = std::array::from_fn(|k| {
        middle[k] - (right[k] * width + up[k] * height + left[k] * depth) / 2.0
    });
    std::array::from_fn(|i| {
        let along = [
            step(right, if i & 1 == 1 { width } else { 0.0 }),
            step(up, if i & 2 == 2 { height } else { 0.0 }),
            step(left, if i & 4 == 4 { depth } else { 0.0 }),
        ];
        std::array::from_fn(|k| origin[k] + along[0][k] + along[1][k] + along[2][k])
    })
}

/// The pinhole eye at the world's origin, its focal length 1, looking along z turned up by
/// `pitch` (radians; down when negative).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Eye {
    pub(crate) pitch: f64,
}

impl Eye {
    /// The eye's view of the world point or direction `p`: right, up, forward.
    fn view(self, p: [f64; 3]) -> [f64; 3] {
        let (sin, cos) = self.pitch.sin_cos();
        [p[0], p[1] * cos - p[2] * sin, p[1] * sin + p[2] * cos]
    }

    /// The world point at `p` in the eye's view (right, up, forward).
    pub(crate) fn to_world(self, p: [f64; 3]) -> [f64; 3] {
        let (sin, cos) = self.pitch.sin_cos();
        [p[0], p[1] * cos + p[2] * sin, -p[1] * sin + p[2] * cos]
    }

    /// Where the world point `p` lies in the image (y down), if in front of the eye.
    pub(crate) fn image(self, p: [f64; 3]) -> Option<Point> {
        let [x, y, z] = self.view(p);
        (z > 0.3).then(|| [x / z, -y / z])
    }

    /// The vanishing point of the direction `d`; none when it lies parallel to the image.
    pub(crate) fn vanishing(self, d: [f64; 3]) -> Option<Point> {
        let [x, y, z] = self.view(d);
        (z.abs() > 1e-9).then(|| [x / z, -y / z])
    }
}

/// The box whose image corners are `image`, horizon and vanishing points `(horizon, vanishing)`,
/// sized so its longer side is `size` and moved into the frame: its middle `shift` off the
/// frame's, or on a page its horizon on the page's and its middle across.
pub(crate) fn fitted(
    image: [Point; 8],
    (horizon, vanishing): (f64, [Option<Point>; 3]),
    (size, shift): (f64, Point),
    page: Option<Horizon>,
) -> PerspectiveBox {
    let low = [0, 1].map(|k| image.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let high = [0, 1].map(|k| image.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max));
    let scale = size / (high[0] - low[0]).max(high[1] - low[1]).max(1e-9);
    let middle = [0, 1].map(|k| (low[k] + high[k]) / 2.0 * scale);
    let offset = match page {
        None => [0.5 + shift[0] - middle[0], 0.5 + shift[1] - middle[1]],
        Some(page) => [0.5 + shift[0] - middle[0], page.height() - horizon * scale],
    };
    let map = |p: Point| [p[0] * scale + offset[0], p[1] * scale + offset[1]];
    let corners = image.map(map);
    let horizon = horizon * scale + offset[1];
    // Clear of the horizon, the front edge's end nearer it, where three faces show (the bottom
    // one above it); across it, the end farther from it, whose edges are steeper to read.
    let above = corners.iter().all(|c| c[1] < horizon);
    let below = corners.iter().all(|c| c[1] > horizon);
    let farther = |at: usize| (corners[at][1] - horizon).abs();
    let front = match (above, below) {
        (true, _) => 0,
        (_, true) => 2,
        _ if farther(0) > farther(2) => 0,
        _ => 2,
    };
    PerspectiveBox {
        corners,
        horizon,
        vanishing: vanishing.map(|p| p.map(map)),
        front,
    }
}
