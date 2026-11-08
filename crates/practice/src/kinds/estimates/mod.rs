//! Estimates on the flat (`Kind::Flat`, S90): a point, a proportion and an angle, their check, and
//! the How of estimates and points in space.

pub(crate) mod demo_points;
pub(crate) mod flat;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use flat::{Estimate, Flat};
use std::sync::Arc;

pub(crate) struct Estimates;

const FACTS: [Facts; 3] = [
    Facts::of(Family::Point, "point", Work::Geometry),
    Facts::of(Family::Proportion, "proportion", Work::Geometry),
    Facts::of(Family::Angle, "angle", Work::Geometry),
];
/// How many estimates a drill holds.
const DRILLED: usize = 5;

impl Described for Estimates {
    fn code(&self) -> u8 {
        11
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Flat(estimate) => Some(place(&Estimate::ALL, &estimate)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Estimate::ALL.get(usize::from(variant))).map(|e| Kind::Flat(*e))
    }

    fn family(&self, kind: Kind) -> Family {
        match kind {
            Kind::Flat(estimate) => estimate.family(),
            _ => Family::Point,
        }
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Flat(estimate) = exercise.kind else {
            return exercise.picture();
        };
        Original::Flat(Flat::generate(estimate, exercise.seed))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::Flat(flat) => marks::givens(flat),
            _ => Vec::new(),
        }
    }

    fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        match original {
            Original::Flat(flat) => Some(marks::FlatCompared::of(flat, copy)),
            _ => None,
        }
    }

    fn how(&self, family: Family) -> Option<Demo> {
        demo_points::estimate(family)
    }

    /// A drill: five of the one estimate.
    fn drill(&self, family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        (Estimate::ALL.into_iter())
            .filter(|estimate| estimate.family() == family)
            .flat_map(|estimate| std::iter::repeat_n((Kind::Flat(estimate), None), DRILLED))
            .collect()
    }
}
