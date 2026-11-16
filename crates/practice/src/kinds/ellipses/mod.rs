//! Ellipses (`Kind::Ellipse`, S84): an ellipse drawn in its frame, the exact one laid over it at
//! Compare for the learner's eye (S145, D123). The guide and its exact ellipse are
//! `geometry/ellipse.rs`; its How `demo.rs`.
//!
//! A lesson (an ellipse about a minute): the first level four, in a plain frame and in a square
//! in perspective by turns, the second five, three of them in perspective, the third five, three
//! in perspective, whose tilt and roundness the plane sets, and two in a plain frame turned
//! (S142), so the ellipse's axes are not the hand's habit.

pub(crate) mod demo;
pub(crate) mod marks;

use crate::course::picker::{laid, turned};
use crate::demos::demo::Demo;
use crate::geometry::ellipse::{EllipseGuide, Guide};
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, curve, line};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use std::sync::Arc;

pub(crate) struct Ellipses;

const FACTS: [Facts; 1] = [Facts::of("ellipse", ToolSet::DRAWING, 60).constructed()];
/// The guides the first level turns through (`Guide::ALL` before S142), and how many it lays; the
/// others' ellipses.
const FIRST_GUIDES: [Guide; 2] = [Guide::Frame, Guide::Plane];
const FIRST: usize = 4;
const SECOND: [Guide; 5] = [
    Guide::Plane,
    Guide::Frame,
    Guide::Plane,
    Guide::Frame,
    Guide::Plane,
];
const THIRD: [Guide; 5] = [
    Guide::Plane,
    Guide::Turned,
    Guide::Plane,
    Guide::Turned,
    Guide::Plane,
];

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
            SkillLevel::First => turned((&FIRST_GUIDES, FIRST), turn, Kind::Ellipse),
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

    /// Once anything lies in the copy's frame, the exact ellipse laid over it for the eye; the
    /// copy's strokes are never read or fitted (D123).
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || marks::EllipseCompared { guide: *self };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }
}
