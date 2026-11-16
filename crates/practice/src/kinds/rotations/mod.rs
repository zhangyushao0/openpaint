//! Structure rotation (`Kind::Rotation`, S113, V大's weeks 5 and 6 结构翻转): an object seen as a
//! few solids in a box well enough to draw it from an angle never seen. One of unit 4's carved
//! objects is shown lit on paper at one angle (view A) and asked drawn at another (view B, its box
//! started in the copy's frame); the answer is the same design turned and seen from B, exact to
//! the last bits, laid over the copy at Compare for the learner's eye to judge (the author,
//! 2026-11-16). Nothing is rendered ahead and nothing is stored but kind and seed (`rotation.rs`).
//!
//! A lesson holds one object (about 6 minutes); the levels turn it more: a quarter, the eye's
//! height kept; from another height, a quarter or a half; from another height and turned with only
//! the new box's upright given beside a small angle box (`angle.rs`).

pub(crate) mod angle;
pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod rotation;

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
use crate::marks::tile::{TileMark, arc, boxed};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use marks::RotationCompared;
use rotation::{Rotation, RotationTask};

pub(crate) struct Rotations;

const FACTS: [Facts; 1] = [Facts::of("structure-rotation", ToolSet::DRAWING, 360)];
/// The variants each level takes in turn, a lesson's place (`Rotation::ALL`'s).
const FIRST: [Rotation; 2] = [Rotation::ALL[0], Rotation::ALL[1]];
const SECOND: [Rotation; 2] = [Rotation::ALL[2], Rotation::ALL[3]];
const THIRD: [Rotation; 1] = [Rotation::ALL[4]];

impl Described for Rotations {
    fn code(&self) -> u8 {
        22
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Rotation(rotation) => Some(place(&Rotation::ALL, &rotation)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Rotation::ALL.get(usize::from(variant))).map(|rotation| Kind::Rotation(*rotation))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Rotation(rotation) = exercise.kind else {
            return held(Blank);
        };
        held(RotationTask::generate(rotation, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        demo::rotation()
    }

    /// One object: the level's variants in turn, a lesson's place.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let variants: &[Rotation] = match level {
            SkillLevel::First => &FIRST,
            SkillLevel::Second => &SECOND,
            SkillLevel::Third => &THIRD,
        };
        laid([Kind::Rotation(variants[turn.turn % variants.len()])])
    }

    fn window(&self) -> usize {
        3
    }

    /// A box, and the same box turned beside it, an arrow over them.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = boxed([-12.5, 3.0], 0.5);
        let mirrored = (boxed([-12.5, 3.0], 0.5).into_iter()).map(|mark| match mark {
            TileMark::Line(points) => {
                TileMark::Line(points.iter().map(|[x, y]| [54.0 - x, *y]).collect())
            }
            other => other,
        });
        marks.extend(mirrored);
        let tip = [37.5, 9.0];
        marks.extend([
            arc([27.0, 12.0], [11.0, 6.0], (195.0, 325.0)),
            TileMark::Line(vec![
                [tip[0] - 3.5, tip[1] - 2.5],
                tip,
                [tip[0] - 1.0, tip[1] + 3.5],
            ]),
        ]);
        marks
    }
}

/// An object to draw turned.
impl Original for RotationTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, B's answer laid over it for the eye, a
    /// Construction to lay with it; the copy's strokes are never read.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || RotationCompared { task: self.clone() };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// View A's picture, `OBJECT_SIDE` square.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([OBJECT_SIDE; 2], self.shown.picture(OBJECT_SIDE)))
    }

    /// A stroke along each edge and seen curve of view B.
    fn answer(&self) -> Vec<Vec<Point>> {
        let answer = &self.asked.answer;
        (answer.sides.iter().map(|side| side.to_vec()))
            .chain(
                (answer.outlines.iter().flatten())
                    .filter(|stretch| !stretch.hidden)
                    .map(|stretch| stretch.points.clone()),
            )
            .collect()
    }
}
