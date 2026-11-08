//! Boxes to complete (`Kind::Box`, V大's 透视盒子): the check of a completed box's edges, and the
//! How of boxes, free boxes and ellipses. The box itself is `geometry/perspective.rs`.

pub(crate) mod demo_space;
pub(crate) mod marks;
pub(crate) mod perspective_check;

use crate::demos::demo::Demo;
use crate::geometry::perspective::{BoxView, PerspectiveBox};
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::drill::turned;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use std::sync::Arc;

pub(crate) struct Boxes;

const FACTS: [Facts; 1] = [Facts::of(Family::Box, "box", Work::Geometry).constructed()];
/// How many boxes a drill holds.
const DRILLED: usize = 3;

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

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Box(view) = exercise.kind else {
            return exercise.picture();
        };
        Original::Box(PerspectiveBox::generate(view, exercise.seed))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::Box(exact) => marks::givens(exact),
            _ => Vec::new(),
        }
    }

    fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        match original {
            Original::Box(exact) => Some(marks::BoxCompared::of(exact, copy)),
            _ => None,
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_space::perspective_box())
    }

    /// A drill: three boxes, their views in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        turned((&BoxView::ALL, DRILLED), turn, Kind::Box)
    }
}
