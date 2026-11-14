//! Boxes to complete (`Kind::Box`, V大's 透视盒子): the check of a completed box's edges, and the
//! How, tile and lessons of boxes. The box itself is `geometry/perspective.rs`.
//!
//! A lesson (a box about 2.5 minutes) holds two: the first level a two-point and a three-point
//! box, their order in turn, the second and the third two
//! three-point boxes, whose third vanishing point is the harder to keep; a box has no third view.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod perspective_check;

use crate::course::picker::{laid, turned};
use crate::demos::demo::Demo;
use crate::geometry::perspective::{BoxView, PerspectiveBox};
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, boxed};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use std::sync::Arc;

pub(crate) struct Boxes;

const FACTS: [Facts; 1] = [Facts::of("box", ToolSet::DRAWING, 150).constructed()];
/// The views the first level turns through, three a lesson's place (stage 2's drills), of which
/// it lays the first `COUNT`.
const TURNED: usize = 3;
const COUNT: usize = 2;

impl Described for Boxes {
    fn code(&self) -> u8 {
        8
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Box(view) => Some(place(&BoxView::ALL, &view)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        BoxView::ALL
            .get(usize::from(variant))
            .map(|v| Kind::Box(*v))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Box(view) = exercise.kind else {
            return held(Blank);
        };
        held(PerspectiveBox::generate(view, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::perspective_box())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        match level {
            SkillLevel::First => {
                let mut boxes = turned((&BoxView::ALL, TURNED), turn, Kind::Box);
                boxes.truncate(COUNT);
                boxes
            }
            SkillLevel::Second | SkillLevel::Third => laid([Kind::Box(BoxView::ThreePoint); COUNT]),
        }
    }

    /// A box seen from above, its front corner nearest.
    fn glyph(&self) -> Vec<TileMark> {
        boxed([0.0, 0.0], 1.0)
    }
}

/// A box to complete: the exact box, its front corner's three edges given.
impl Original for PerspectiveBox {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::BoxCompared::of(self, copy))
    }
}
