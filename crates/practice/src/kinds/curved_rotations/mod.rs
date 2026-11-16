//! Curved structure rotation (`Kind::CurvedRotation`, S154f, V大's weeks 7 and 8 曲面翻转): a curved
//! object split into hemispheres, cylinders and cones, each joint's section placed in a box well
//! enough to draw it from an angle never seen. A body turned about one axis (`design.rs`) is shown
//! lit on paper one way (view A) and asked drawn another (view B, its box started in the copy's
//! frame, its axis dashed through the box's end faces' middles); the answer is the same body seen
//! from B, exact (`answer.rs`), laid over the copy at Compare for the learner's eye to judge (D123).
//! Nothing is rendered ahead and nothing is stored but kind and seed (`curved.rs`).
//!
//! A lesson holds one object (about 6 minutes); the levels are V大's three stages from precise to
//! estimated: the joints ticked on the axis; the axis alone, a spout or a handle on the side; only
//! the axis, the box's near upright and a small box at the angle.

pub(crate) mod answer;
pub(crate) mod attached;
pub(crate) mod body;
pub(crate) mod curved;
pub(crate) mod demo;
pub(crate) mod design;
pub(crate) mod marks;
pub(crate) mod picture;
pub(crate) mod sections;
pub(crate) mod view;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::objects::OBJECT_SIDE;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, arc};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use curved::{CurvedRotation, CurvedTask};
use marks::CurvedCompared;

pub(crate) struct CurvedRotations;

const FACTS: [Facts; 1] = [Facts::of("curved-rotation", ToolSet::DRAWING, 360)];
/// The variants each level takes in turn, a lesson's place (`CurvedRotation::ALL`'s).
const FIRST: [CurvedRotation; 2] = [CurvedRotation::ALL[0], CurvedRotation::ALL[1]];
const SECOND: [CurvedRotation; 2] = [CurvedRotation::ALL[2], CurvedRotation::ALL[3]];
const THIRD: [CurvedRotation; 2] = [CurvedRotation::ALL[4], CurvedRotation::ALL[5]];

impl Described for CurvedRotations {
    fn code(&self) -> u8 {
        28
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::CurvedRotation(curved) => Some(place(&CurvedRotation::ALL, &curved)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (CurvedRotation::ALL.get(usize::from(variant))).map(|curved| Kind::CurvedRotation(*curved))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::CurvedRotation(curved) = exercise.kind else {
            return held(Blank);
        };
        held(CurvedTask::generate(curved, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        demo::curved()
    }

    /// One object: the level's variants in turn, a lesson's place.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let variants: &[CurvedRotation] = match level {
            SkillLevel::First => &FIRST,
            SkillLevel::Second => &SECOND,
            SkillLevel::Third => &THIRD,
        };
        laid([Kind::CurvedRotation(variants[turn.turn % variants.len()])])
    }

    fn window(&self) -> usize {
        3
    }

    /// A bottle lying, an arrow, the same bottle standing.
    fn glyph(&self) -> Vec<TileMark> {
        let line = |points: &[[f32; 2]]| TileMark::Line(points.to_vec());
        vec![
            // Lying: its body, shoulder and neck along the tile, its foot's ellipse.
            line(&[[4.0, 22.0], [14.0, 22.0], [19.0, 25.0], [22.0, 25.0]]),
            line(&[
                [4.0, 34.0],
                [14.0, 34.0],
                [19.0, 31.0],
                [22.0, 31.0],
                [22.0, 25.0],
            ]),
            arc([4.0, 28.0], [2.0, 6.0], (90.0, 270.0)),
            // The arrow.
            arc([27.0, 21.0], [5.0, 4.0], (200.0, 340.0)),
            line(&[[29.5, 16.5], [31.6, 19.6], [28.0, 20.5]]),
            // Standing: the same, its foot's ellipse and its sections.
            line(&[[37.0, 35.0], [37.0, 22.0], [41.0, 16.0], [41.0, 9.0]]),
            line(&[[50.0, 35.0], [50.0, 22.0], [46.0, 16.0], [46.0, 9.0]]),
            arc([43.5, 35.0], [6.5, 2.0], (0.0, 180.0)),
            arc([43.5, 22.0], [6.5, 2.0], (0.0, 180.0)),
            arc([43.5, 9.0], [2.5, 1.0], (0.0, 360.0)),
        ]
    }
}

/// A curved object to draw from another angle.
impl Original for CurvedTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, B's answer laid over it for the eye, a
    /// construction to lay with it; the copy's strokes are never read.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || CurvedCompared { task: self.clone() };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// View A's picture, `OBJECT_SIDE` square.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([OBJECT_SIDE; 2], self.shown.picture(OBJECT_SIDE)))
    }

    /// A stroke along each edge and outline view B shows.
    fn answer(&self) -> Vec<Vec<Point>> {
        let answer = &self.asked.answer;
        (answer.sides.iter().map(|side| side.to_vec()))
            .chain(
                (answer.curves.iter().flat_map(|curve| &curve.stretches))
                    .filter(|stretch| !stretch.hidden)
                    .map(|stretch| stretch.points.clone()),
            )
            .collect()
    }
}
