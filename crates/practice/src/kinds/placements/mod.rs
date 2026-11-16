//! Placement from a floor plan (`Kind::Placement`, S153, V大's 分布 of weeks 7 and 8: the plan
//! seen from above laid on the box's floor, the heights pulled up along the perspective lines): a
//! few pieces put on a ground in perspective where a plan says, at their heights. The plan and its
//! front view are printed in the original's frame (`printed.rs`); the ground's corners, the horizon
//! and the unit in the copy's; the answer is the pieces in space seen by the eye, exact to the last
//! bits, laid over the copy at Compare for the learner's eye to judge (D123). Nothing is rendered
//! ahead and nothing is stored but kind and seed (`placement.rs`).
//!
//! A lesson holds one placement (about 6 minutes); the levels: two or three boxes on the ground's
//! halves and quarters, a unit or a half tall; four pieces, one turned in the plan and one a
//! cylinder, heights in thirds; five boxes on eighths seen from above in three points, only the
//! unit along the ground's near edge given.

pub(crate) mod answer;
pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod placement;
pub(crate) mod plan;
pub(crate) mod printed;
pub(crate) mod sight;

use std::sync::Arc;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::TileMark;
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use marks::PlacementCompared;
use placement::{Placement, PlacementTask};

pub(crate) struct Placements;

const FACTS: [Facts; 1] = [Facts::of("placement", ToolSet::DRAWING, 360)];
/// The variants each level takes in turn, a lesson's place (`Placement::ALL`'s).
const FIRST: [Placement; 2] = [Placement::ALL[0], Placement::ALL[1]];
const SECOND: [Placement; 1] = [Placement::ALL[2]];
const THIRD: [Placement; 1] = [Placement::ALL[3]];

impl Described for Placements {
    fn code(&self) -> u8 {
        25
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Placement(placement) => Some(place(&Placement::ALL, &placement)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Placement::ALL.get(usize::from(variant))).map(|placement| Kind::Placement(*placement))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Placement(placement) = exercise.kind else {
            return held(Blank);
        };
        held(PlacementTask::generate(placement, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::placement())
    }

    /// One placement: the level's variants in turn, a lesson's place.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let variants: &[Placement] = match level {
            SkillLevel::First => &FIRST,
            SkillLevel::Second => &SECOND,
            SkillLevel::Third => &THIRD,
        };
        laid([Kind::Placement(variants[turn.turn % variants.len()])])
    }

    fn window(&self) -> usize {
        3
    }

    /// A ground in perspective, a box standing at its back and a lower one at its front.
    fn glyph(&self) -> Vec<TileMark> {
        let line = |points: &[[f32; 2]]| TileMark::Line(points.to_vec());
        vec![
            line(&[
                [25.0, 35.0],
                [49.0, 27.0],
                [32.0, 20.0],
                [6.0, 26.0],
                [25.0, 35.0],
            ]),
            // The front box, low.
            line(&[[20.0, 30.0], [20.0, 26.0], [27.0, 24.0], [27.0, 28.0]]),
            line(&[[20.0, 26.0], [15.0, 24.5], [22.0, 22.5], [27.0, 24.0]]),
            line(&[[15.0, 24.5], [15.0, 28.5], [20.0, 30.0]]),
            // The back box, tall.
            line(&[[34.0, 25.0], [34.0, 14.0], [40.0, 12.5], [40.0, 23.0]]),
            line(&[[34.0, 14.0], [29.0, 12.5], [35.0, 11.0], [40.0, 12.5]]),
            line(&[[29.0, 12.5], [29.0, 23.5], [34.0, 25.0]]),
        ]
    }
}

/// A plan to place on its ground.
impl Original for PlacementTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, the answer laid over it for the eye, a
    /// Construction to lay with it; the copy's strokes are never read.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || PlacementCompared { task: self.clone() };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// A stroke along each edge and outline the eye sees.
    fn answer(&self) -> Vec<Vec<Point>> {
        (self.answer.sides.iter().map(|side| side.to_vec()))
            .chain(
                (self.answer.outlines.iter().flatten())
                    .filter(|stretch| !stretch.hidden)
                    .map(|stretch| stretch.points.clone()),
            )
            .collect()
    }
}
