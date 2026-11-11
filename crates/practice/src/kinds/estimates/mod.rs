//! Estimates on the flat (`Kind::Flat`, S90): a point, a proportion and an angle, their check,
//! their How (`demo.rs`), tiles and answers as a hand draws them.

pub(crate) mod demo;
pub(crate) mod flat;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::demos::demo_points::crossed;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, line, square};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use flat::{Estimate, Flat, FlatAnswer};
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

    fn key(&self) -> &'static str {
        "estimates"
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
        demo::estimate(family)
    }

    /// A drill: five of the one estimate.
    fn drill(&self, family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        (Estimate::ALL.into_iter())
            .filter(|estimate| estimate.family() == family)
            .flat_map(|estimate| std::iter::repeat_n((Kind::Flat(estimate), None), DRILLED))
            .collect()
    }

    /// A small cross on a point, a stroke along a line.
    fn answer(&self, original: &Original) -> Vec<Vec<Point>> {
        let Original::Flat(flat) = original else {
            return Vec::new();
        };
        match flat.answer() {
            FlatAnswer::Point(at) => crossed(at).to_vec(),
            FlatAnswer::Line(line) => vec![line.to_vec()],
        }
    }

    /// A dot in a frame, a segment its first end ringed and its last barred with a dot along it,
    /// a slanted line through a dot.
    fn tile(&self, family: Family) -> Vec<TileMark> {
        match family {
            Family::Point => vec![
                line(&closed(&square([14.0, 7.0], [40.0, 33.0]))),
                TileMark::Dot([31.0, 15.0]),
            ],
            Family::Proportion => vec![
                line(&[[10.0, 28.0], [45.0, 14.0]]),
                TileMark::Ring([10.0, 28.0]),
                line(&[[44.0, 11.5], [46.0, 16.5]]),
                TileMark::Dot([23.3, 22.7]),
            ],
            _ => vec![
                line(&[[14.0, 33.0], [40.0, 7.0]]),
                TileMark::Dot([27.0, 20.0]),
            ],
        }
    }
}
