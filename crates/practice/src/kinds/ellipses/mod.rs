//! Ellipses (`Kind::Ellipse`, S84): the fit of a hand's ellipse and its check. The guide and its
//! exact ellipse are `geometry/ellipse.rs`; the How is the boxes' (`demo_space.rs`).

pub(crate) mod ellipse_check;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::geometry::ellipse::{EllipseGuide, Guide};
use crate::kinds::boxes::demo_space;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::drill::turned;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use std::sync::Arc;

pub(crate) struct Ellipses;

const FACTS: [Facts; 1] = [Facts::of(Family::Ellipse, "ellipse", Work::Geometry).constructed()];
/// How many ellipses a drill holds.
const DRILLED: usize = 4;

impl Described for Ellipses {
    fn code(&self) -> u8 {
        10
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Ellipse(guide) => Some(place(&Guide::ALL, &guide)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Guide::ALL
            .get(usize::from(variant))
            .map(|g| Kind::Ellipse(*g))
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Ellipse(guide) = exercise.kind else {
            return exercise.picture();
        };
        Original::Ellipse(EllipseGuide::generate(guide, exercise.seed))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::Ellipse(guide) => marks::givens(guide),
            _ => Vec::new(),
        }
    }

    fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        match original {
            Original::Ellipse(guide) => Some(marks::EllipseCompared::of(guide, copy)),
            _ => None,
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_space::ellipse())
    }

    /// A drill: four ellipses, their guides in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        turned((&Guide::ALL, DRILLED), turn, Kind::Ellipse)
    }
}
