//! Form intersections (`Kind::Intersection`, S156k, Drawabox's form intersections, V大's weeks 7 and
//! 8, 穿插): finding the line along which one form passes into another, its pierce points first,
//! then the line between them, which fixes how the two stand in space. Two forms are laid in one
//! box's perspective (`placing.rs`, `forms.rs`) and printed in the one frame; the answer is where
//! they meet, exact (`pairs.rs`, `track.rs`), seen and hidden by the depth test (`meeting.rs`),
//! laid over the copy at Compare for the learner's eye to judge (D123). Nothing is rendered ahead
//! and nothing is stored but kind and seed.
//!
//! A lesson holds two pairs (about 3 minutes each); the levels: a box into a box, both drawn
//! through; a cylinder through a box's face or a box into a cylinder's side; two cylinders or a
//! ball sunk into a cylinder, only what the eye sees of them printed. Each level is a topic of its
//! own, its How on an example of its forms (S158k): pierce points on the boxes, a cylinder's
//! ellipse in its square on a box's face, sections across two cylinders.

pub(crate) mod demo;
pub(crate) mod forms;
pub(crate) mod marks;
pub(crate) mod meeting;
pub(crate) mod pairs;
pub(crate) mod placing;
pub(crate) mod track;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, arc, boxed};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use marks::IntersectionCompared;
use meeting::{IntersectionTask, Meeting};

pub(crate) struct Intersections;

/// Its topics: two boxes (入门), a cylinder and a box (熟练), two round forms (稳).
const FACTS: [Facts; 3] = [
    Facts::of("intersections", ToolSet::DRAWING, 180).constructed(),
    Facts::of("intersections-cylinder", ToolSet::DRAWING, 180).constructed(),
    Facts::of("intersections-round", ToolSet::DRAWING, 180).constructed(),
];
/// The variants each level takes in turn, a lesson's two in their order.
const FIRST: [Meeting; 2] = [Meeting::ALL[0], Meeting::ALL[1]];
const SECOND: [Meeting; 2] = [Meeting::ALL[2], Meeting::ALL[3]];
const THIRD: [Meeting; 2] = [Meeting::ALL[4], Meeting::ALL[5]];

impl Described for Intersections {
    fn code(&self) -> u8 {
        35
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Intersection(meeting) => Some(place(&Meeting::ALL, &meeting)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Meeting::ALL.get(usize::from(variant))).map(|meeting| Kind::Intersection(*meeting))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        match kind {
            Kind::Intersection(Meeting::CylinderIntoBox | Meeting::BoxIntoCylinder) => &FACTS[1],
            Kind::Intersection(Meeting::Cylinders | Meeting::Sphere) => &FACTS[2],
            _ => &FACTS[0],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Intersection(meeting) = exercise.kind else {
            return held(Blank);
        };
        held(IntersectionTask::generate(meeting, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Intersection(meeting) = kind else {
            return None;
        };
        Some(demo::intersections(meeting))
    }

    /// Two pairs, the level's variants, the first first on even turns.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let variants = match level {
            SkillLevel::First => FIRST,
            SkillLevel::Second => SECOND,
            SkillLevel::Third => THIRD,
        };
        let first = turn.turn % 2;
        laid([variants[first], variants[1 - first]].map(Kind::Intersection))
    }

    /// A box with a cylinder through its face, the ellipse where they meet on the face.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = boxed([-6.0, 2.0], 0.8);
        marks.extend([
            arc([27.4, 23.2], [2.4, 5.0], (0.0, 360.0)),
            TileMark::Line(vec![[27.4, 18.2], [47.0, 15.6]]),
            TileMark::Line(vec![[27.4, 28.2], [47.0, 25.6]]),
            arc([47.0, 20.6], [2.4, 5.0], (0.0, 360.0)),
        ]);
        marks
    }
}

/// Two forms to find the meeting of.
impl Original for IntersectionTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the frame, the exact line where the forms meet laid over it for
    /// the eye, a construction to lay with it; the copy's strokes are never read.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || IntersectionCompared { task: self.clone() };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// A stroke along each stretch of the meeting line the eye sees.
    fn answer(&self) -> Vec<Vec<Point>> {
        (self.answer.iter())
            .filter(|stretch| !stretch.hidden)
            .map(|stretch| stretch.points.clone())
            .collect()
    }
}
