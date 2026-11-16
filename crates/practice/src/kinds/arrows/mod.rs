//! Organic arrows (`Kind::Arrow`, S156l, Drawabox's arrows; V大's 近大远小 on a free form): a flat
//! ribbon flowing back through space, drawn along a path printed in its one frame: narrower and
//! its bends closer far away, the nearer stretch hiding the farther where they cross, turning over
//! to show its back. The ribbon is generated (`ribbon.rs`), its answer exact for the sampled ribbon
//! (`answer.rs`), laid over the copy at Compare for the learner's eye to judge (D123): the copy's
//! strokes are never read. Nothing is stored but kind and seed (`task.rs`).
//!
//! A lesson holds two arrows (about 2.5 minutes each); the levels: a gentle weave, no crossing and
//! the far width given; a tighter weave going farther back, its far stretches crossing behind its
//! near ones; crossings and one or two half turns.

pub(crate) mod answer;
pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod ribbon;
pub(crate) mod task;

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
use crate::marks::tile::{TileMark, closed, line};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use marks::ArrowCompared;
use ribbon::Arrow;
use task::ArrowTask;

pub(crate) struct Arrows;

const FACTS: [Facts; 1] = [Facts::of("organic-arrow", ToolSet::DRAWING, 150).constructed()];
/// The arrows a lesson lays.
const ARROWS: usize = 2;

impl Described for Arrows {
    fn code(&self) -> u8 {
        33
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Arrow(arrow) => Some(place(&Arrow::ALL, &arrow)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Arrow::ALL.get(usize::from(variant))).map(|arrow| Kind::Arrow(*arrow))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Arrow(arrow) = exercise.kind else {
            return held(Blank);
        };
        held(ArrowTask::generate(arrow, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Arrow(arrow) = kind else {
            return None;
        };
        Some(demo::arrow(arrow))
    }

    /// Two arrows of the level's variant.
    fn lesson(&self, level: SkillLevel, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let arrow = match level {
            SkillLevel::First => Arrow::Gentle,
            SkillLevel::Second => Arrow::Crossing,
            SkillLevel::Third => Arrow::Turning,
        };
        laid([Kind::Arrow(arrow); ARROWS])
    }

    /// A ribbon weaving back from its arrowhead, narrowing, a far stretch passing behind a near.
    fn glyph(&self) -> Vec<TileMark> {
        vec![
            line(&closed(&[[10.0, 22.0], [4.0, 28.0], [10.0, 34.0]])),
            line(&[[10.0, 24.0], [36.0, 22.0], [18.0, 15.0], [42.0, 9.0]]),
            line(&[[10.0, 33.0], [36.0, 30.0], [36.0, 22.0]]),
            line(&[[18.0, 15.0], [18.0, 21.0], [22.1, 23.1]]),
            line(&[[26.3, 18.2], [42.0, 13.0], [42.0, 9.0]]),
        ]
    }
}

/// A ribbon to draw along its path.
impl Original for ArrowTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the frame, the ribbon laid over it for the eye, its widths to lay
    /// with it; the copy's strokes are never read.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || ArrowCompared {
            answer: self.answer.clone(),
        };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// A stroke along each seen stretch of the edges and the folds.
    fn answer(&self) -> Vec<Vec<Point>> {
        let answer = &self.answer;
        (answer.stretches.iter().flatten())
            .chain(answer.folds.iter().flatten())
            .filter(|stretch| !stretch.hidden)
            .map(|stretch| stretch.points.clone())
            .collect()
    }
}
