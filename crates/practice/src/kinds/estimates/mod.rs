//! Estimates on the flat (`Kind::Flat`, S90): a point, a proportion and an angle, their check,
//! their How (`demo.rs`), tiles, lessons and answers as a hand draws them.
//!
//! A lesson (an estimate about 36 seconds) holds six, its topics in the course's order: the first
//! level points in a frame, the second proportions along a segment with two points among them,
//! the third angles mixed with the other two.

pub(crate) mod demo;
pub(crate) mod flat;
pub(crate) mod marks;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::demos::demo_points::crossed;
use crate::geometry::heading::ON_ANGLE;
use crate::geometry::off::PEN_WIDTH;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, line, square};
use crate::maths::path::{Point, distance};
use crate::packs::pack::ItemRef;
use crate::progress::scaled;
use crate::records::metric::{Metric, Metrics, value};
use crate::sheets::exercise::Exercise;
use flat::{Estimate, Flat, FlatAnswer, NEAR_ANGLE, NEAR_OFF};
use std::sync::Arc;

pub(crate) struct Estimates;

/// Its topics, in `Estimate::ALL`'s order.
const FACTS: [Facts; 3] = [
    Facts::of("point", ToolSet::DRAWING, 36),
    Facts::of("proportion", ToolSet::DRAWING, 36),
    Facts::of("angle", ToolSet::DRAWING, 36),
];
/// Each level's estimates.
const FIRST: [Estimate; 6] = [Estimate::Point; 6];
const SECOND: [Estimate; 6] = [
    Estimate::Proportion,
    Estimate::Point,
    Estimate::Proportion,
    Estimate::Proportion,
    Estimate::Point,
    Estimate::Proportion,
];
const THIRD: [Estimate; 6] = [
    Estimate::Angle,
    Estimate::Proportion,
    Estimate::Angle,
    Estimate::Point,
    Estimate::Angle,
    Estimate::Proportion,
];

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

    fn topic(&self, kind: Kind) -> &'static Facts {
        let Kind::Flat(estimate) = kind else {
            return &FACTS[0];
        };
        &FACTS[usize::from(place(&Estimate::ALL, &estimate))]
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Flat(estimate) = exercise.kind else {
            return held(Blank);
        };
        held(Flat::generate(estimate, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Flat(estimate) = kind else {
            return None;
        };
        demo::estimate(estimate)
    }

    /// A point's distance off, a proportion's along its segment (the task made again from its
    /// seed), each cut at a pen's width and `NEAR_OFF`; an angle's turn cut at `ON_ANGLE` and
    /// `NEAR_ANGLE`.
    fn measured(&self, exercise: &Exercise, metrics: &Metrics) -> Option<f64> {
        let Kind::Flat(estimate) = exercise.kind else {
            return None;
        };
        let near = [PEN_WIDTH, NEAR_OFF];
        Some(match Flat::generate(estimate, exercise.seed) {
            Flat::Point { .. } => {
                let across = value(metrics, Metric::PointAcross)?;
                scaled(across.hypot(value(metrics, Metric::PointDown)?), near)
            }
            Flat::Proportion { copy, .. } => {
                let along = value(metrics, Metric::ProportionOff)?.abs();
                scaled(along * distance(copy[0], copy[1]), near)
            }
            Flat::Angle { .. } => {
                let turned = value(metrics, Metric::AngleOff)?.abs();
                scaled(turned, [ON_ANGLE, NEAR_ANGLE])
            }
        })
    }

    fn lesson(&self, level: SkillLevel, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let estimates = match level {
            SkillLevel::First => FIRST,
            SkillLevel::Second => SECOND,
            SkillLevel::Third => THIRD,
        };
        laid(estimates.map(Kind::Flat))
    }

    /// A dot in a frame.
    fn glyph(&self) -> Vec<TileMark> {
        vec![
            line(&closed(&square([14.0, 7.0], [40.0, 33.0]))),
            TileMark::Dot([31.0, 15.0]),
        ]
    }
}

/// An estimate's givens: its answer follows from them.
impl Original for Flat {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::FlatCompared::of(self, copy))
    }

    /// A small cross on a point, a stroke along a line.
    fn answer(&self) -> Vec<Vec<Point>> {
        match Flat::answer(self) {
            FlatAnswer::Point(at) => crossed(at).to_vec(),
            FlatAnswer::Line(line) => vec![line.to_vec()],
        }
    }
}
