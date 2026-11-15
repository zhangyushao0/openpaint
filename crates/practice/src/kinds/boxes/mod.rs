//! Boxes to complete (`Kind::Box`, V大's 透视盒子): the check of a completed box's edges, and the
//! How, tile and lessons of boxes. The box itself is `geometry/perspective/`.
//!
//! A lesson (a box about 2.5 minutes) holds two: the first level a two-point and a three-point
//! box, their order in turn; the second a three-point box and a long one (S142, V大's long, flat
//! or tall), their order in turn; the third a box drawn through, its hidden edges checked too
//! (Drawabox), and a long or a three-point box in turn, each of the two first in turn.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod perspective_check;

use crate::course::picker::{laid, rotated, turned};
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
/// The views the first level turns through (`BoxView::ALL` before S142), three a lesson's place
/// (unit 2's drills before S122), of which it lays the first `COUNT`.
const FIRST: [BoxView; 2] = [BoxView::TwoPoint, BoxView::ThreePoint];
const TURNED: usize = 3;
const COUNT: usize = 2;
/// The second level's views, their order in turn; the third's: a box drawn through with each of
/// `BESIDE` in turn.
const SECOND: [BoxView; 2] = [BoxView::ThreePoint, BoxView::Long];
const BESIDE: [BoxView; 2] = [BoxView::Long, BoxView::ThreePoint];

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
        let exact = PerspectiveBox::generate(view, exercise.seed);
        match view {
            BoxView::Through => held(ThroughBox(exact)),
            BoxView::TwoPoint | BoxView::ThreePoint | BoxView::Long => held(exact),
        }
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::perspective_box())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let views = match level {
            SkillLevel::First => {
                let mut boxes = turned((&FIRST, TURNED), turn, Kind::Box);
                boxes.truncate(COUNT);
                return boxes;
            }
            SkillLevel::Second => rotated(&SECOND, turn.turn, COUNT),
            SkillLevel::Third => {
                let through = [BoxView::Through, BESIDE[turn.turn % BESIDE.len()]];
                rotated(&through, turn.turn, COUNT)
            }
        };
        laid(views.into_iter().map(Kind::Box))
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

/// A box to draw through (S142): given as any box to complete, complete only with its three
/// hidden edges too.
#[derive(Debug)]
pub struct ThroughBox(pub PerspectiveBox);

impl Original for ThroughBox {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(&self.0)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::BoxCompared::through(&self.0, copy))
    }
}
