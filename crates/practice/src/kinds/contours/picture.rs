//! A sausage's picture (S156k): the tube lit on paper as unit 4's objects are (the nearest surface
//! on each ray, lit from one side above, no lines), drawn by the shared lit picture
//! (`objects/object_picture.rs`) from the same capsules every line of the exercise lies on: shade
//! and overlap show which way it turns.

use crate::demos::demo::Pictured;
use crate::kinds::contours::task::ContourTask;
use crate::kinds::objects::object_picture::lit_picture;
use crate::maths::path::Point;
use crate::maths::vector::dot;

/// The light: what every surface shows and what the light adds facing it; the clay's tint.
const AMBIENT: f64 = 0.16;
const DIFFUSE: f64 = 0.74;
const TINT: [f64; 3] = [1.0, 0.965, 0.92];

impl ContourTask {
    /// The capsule the eye sees at the frame point `p` and its shade (0 black, 1 white); none
    /// where it sees the paper.
    pub(crate) fn surface_at(&self, p: Point) -> Option<(u32, f64)> {
        let x = (p[0] - self.view.offset[0]) / self.view.scale;
        let y = (p[1] - self.view.offset[1]) / self.view.scale;
        let (hit, _) = self.sausage.cast(([0.0; 3], [x, -y, 1.0]))?;
        Some((
            hit.surface,
            AMBIENT + DIFFUSE * dot(hit.normal, self.light).max(0.0),
        ))
    }

    /// The picture `side` pixels square, one to its frame: RGBA, the colour not multiplied by the
    /// alpha (the paper left clear).
    #[must_use]
    pub fn picture(&self, side: u32) -> Vec<u8> {
        lit_picture((side, self.extent, TINT), |p| self.surface_at(p))
    }
}

/// An example's picture, as its exercise's original frame shows it.
impl Pictured for ContourTask {
    fn picture(&self, side: u32) -> Vec<u8> {
        ContourTask::picture(self, side)
    }

    fn same(&self, other: &dyn std::any::Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }
}
