//! Ellipses (`Kind::Ellipse`, S84): the fit of a hand's ellipse and its check. The guide and its
//! exact ellipse are `geometry/ellipse.rs`; its How `demo.rs`.
//!
//! A lesson (an ellipse about a minute): the first level four, in a plain frame and in a square
//! in perspective by turns (stage 2's drills), the second five, three of them in perspective,
//! the third five in perspective, whose tilt and roundness the plane sets.

pub(crate) mod demo;
pub(crate) mod ellipse_check;
pub(crate) mod marks;

use crate::course::picker::{laid, turned};
use crate::demos::demo::Demo;
use crate::geometry::ellipse::{EllipseGuide, Guide};
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, curve, line};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use std::sync::Arc;

pub(crate) struct Ellipses;

const FACTS: [Facts; 1] = [Facts::of("ellipse", ToolSet::DRAWING, 60).constructed()];
/// The first level's ellipses; the others'.
const FIRST: usize = 4;
const SECOND: [Guide; 5] = [
    Guide::Plane,
    Guide::Frame,
    Guide::Plane,
    Guide::Frame,
    Guide::Plane,
];
const THIRD: [Guide; 5] = [Guide::Plane; 5];

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

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Ellipse(guide) = exercise.kind else {
            return held(Blank);
        };
        held(EllipseGuide::generate(guide, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::ellipse())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        match level {
            SkillLevel::First => turned((&Guide::ALL, FIRST), turn, Kind::Ellipse),
            SkillLevel::Second => laid(SECOND.map(Kind::Ellipse)),
            SkillLevel::Third => laid(THIRD.map(Kind::Ellipse)),
        }
    }

    /// An ellipse in a square in perspective.
    fn glyph(&self) -> Vec<TileMark> {
        vec![
            line(&closed(&[
                [12.0, 31.0],
                [42.0, 31.0],
                [36.0, 11.0],
                [18.0, 11.0],
            ])),
            TileMark::Line(curve(|t| {
                let turn = std::f32::consts::TAU * t;
                [27.0 + 12.6 * turn.cos(), 21.5 + 8.6 * turn.sin()]
            })),
        ]
    }
}

/// An ellipse's guide and the exact ellipse.
impl Original for EllipseGuide {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::EllipseCompared::of(self, copy))
    }
}
