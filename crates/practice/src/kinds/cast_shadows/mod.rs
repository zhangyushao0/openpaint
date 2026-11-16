//! Cast shadows (`Kind::Shadows`, S153, 投影, unit 6): where a box's shadow falls on the ground in
//! perspective from a given light, each shadow corner where the light's ray through a top corner
//! meets the ground line through the corner under it. Boxes stand on the ground in a light given by
//! a pin or a lamp (`scene.rs`), kept to the scene rules (`rules.rs`); the answer is the shadows'
//! exact outline (`answer.rs`, a shadow climbing a side `climb.rs`), laid over the copy at Compare
//! for the learner's eye with each corner's construction under Construction (D123: the copy's
//! strokes are never read).
//!
//! A lesson holds two scenes (about 2.5 minutes each): at 入门 a sun parallel to the picture; at
//! 熟练 a sun beyond the picture and one behind the eye, their order in turn; at 稳 a lamp, the
//! second scene's nearer box's shadow climbing the farther's side.

pub(crate) mod answer;
mod climb;
pub(crate) mod demo;
pub(crate) mod marks;
mod rules;
pub(crate) mod scene;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::{laid, rotated};
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, boxed};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use marks::ShadowCompared;
use scene::{ShadowTask, Throw};

pub(crate) struct CastShadows;

const FACTS: [Facts; 1] = [Facts::of("cast-shadows", ToolSet::DRAWING, 150).constructed()];
/// The scenes each level lays (`Throw::ALL`'s).
const FIRST: [Throw; 2] = [Throw::Side, Throw::Side];
const SECOND: [Throw; 2] = [Throw::Front, Throw::Behind];
const THIRD: [Throw; 2] = [Throw::Lamp, Throw::Climbing];

impl Described for CastShadows {
    fn code(&self) -> u8 {
        27
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Shadows(throw) => Some(place(&Throw::ALL, &throw)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Throw::ALL.get(usize::from(variant))).map(|throw| Kind::Shadows(*throw))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Shadows(throw) = exercise.kind else {
            return held(Blank);
        };
        held(ShadowTask::generate(throw, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::cast_shadows())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let throws = match level {
            SkillLevel::First => FIRST.to_vec(),
            SkillLevel::Second => rotated(&SECOND, turn.turn, SECOND.len()),
            SkillLevel::Third => THIRD.to_vec(),
        };
        laid(throws.into_iter().map(Kind::Shadows))
    }

    /// A box on the ground, its shadow's outline running from its feet along the ground.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = boxed([4.0, 0.0], 0.62);
        let shadow = vec![[18.5, 27.0], [6.0, 33.0], [16.0, 37.0], [31.0, 34.5]];
        marks.insert(0, TileMark::Tint(shadow.clone(), 0.35));
        marks.push(TileMark::Line(shadow));
        marks
    }
}

/// Boxes whose shadows are drawn.
impl Original for ShadowTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, the shadows' outline laid over it for the eye; the
    /// copy's strokes are never read (D123).
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || ShadowCompared { task: self.clone() };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// A stroke along each piece of the outline the eye sees.
    fn answer(&self) -> Vec<Vec<Point>> {
        (self.shadows.outline.iter())
            .filter(|(_, hidden)| !hidden)
            .map(|(ends, _)| ends.to_vec())
            .collect()
    }
}
