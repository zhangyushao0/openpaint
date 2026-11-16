//! A curved object's picture (S154f): the view the learner reads it from, lit on paper as unit 4's
//! objects are (the nearest surface on each ray, lit from one side above, no lines, no horizon),
//! drawn by the shared lit picture (`objects/object_picture.rs`) from the same parts the answer is
//! (`body.rs`): a curved side shaded by its turn, each flat end and face as it faces the light.

use crate::demos::demo::Pictured;
use crate::kinds::curved_rotations::view::CurvedView;
use crate::kinds::objects::object_picture::lit_picture;
use crate::maths::path::Point;
use crate::maths::vector::dot;

/// The light: what every surface shows and what the light adds facing it; the clay's tint.
const AMBIENT: f64 = 0.16;
const DIFFUSE: f64 = 0.72;
const TINT: [f64; 3] = [1.0, 0.965, 0.92];

impl CurvedView {
    /// The surface the eye sees at the frame point `p` and its shade (0 black, 1 white); none
    /// where it sees the paper.
    pub(crate) fn surface_at(&self, p: Point) -> Option<(u32, f64)> {
        let x = (p[0] - self.view.offset[0]) / self.view.scale;
        let y = (p[1] - self.view.offset[1]) / self.view.scale;
        let hit = self.parts.hit([x, -y, 1.0])?;
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
impl Pictured for CurvedView {
    fn picture(&self, side: u32) -> Vec<u8> {
        CurvedView::picture(self, side)
    }

    fn same(&self, other: &dyn std::any::Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }
}
