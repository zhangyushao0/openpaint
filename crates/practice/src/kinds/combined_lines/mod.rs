//! Combined lines (`Kind::Combined`): two to six lines copied as one shape, in six arrangements;
//! given, compared and fitted as a single line's lines are (`lines/marks.rs`).
//!
//! A lesson holds three groups (one of two to four lines about 2 minutes, a larger one of four to
//! six about 2.5, S158): the first level joined, crossing and branched lines (the First-prefix
//! rule), the second near lines (whose gaps are the proportions to keep), lines inside a closed
//! shape (whose empty shapes are) and branched ones, in turn by the lesson; the third two larger
//! groups (S141), a cluster and lines inside a closed shape, their order in turn.

pub(crate) mod combined;
pub(crate) mod demo;

use std::sync::Arc;

use crate::course::picker::{laid, rotated};
use crate::demos::demo::Demo;
use crate::kinds::lines::marks::GivenLines;
use crate::kinds::{
    Blank, Counted, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::tile::{TileMark, curve};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use combined::Combination;

pub(crate) struct CombinedLines;

const FACTS: [Facts; 1] = [Facts::of("combined", ToolSet::DRAWING, 120)
    .aligned()
    .counted(Counted::Line)];
/// About how long a larger group takes (lines inside a closed shape, a cluster: four to six lines).
const LARGER: u16 = 150;
/// Each level's arrangements: the first's in order, the others' in turn from the lesson's place,
/// all of them a lesson.
const FIRST: [Combination; 3] = [
    Combination::Joined,
    Combination::Crossing,
    Combination::Branched,
];
const SECOND: [Combination; 3] = [
    Combination::Near,
    Combination::Enclosed,
    Combination::Branched,
];
const THIRD: [Combination; 2] = [Combination::Cluster, Combination::Enclosed];

impl Described for CombinedLines {
    fn code(&self) -> u8 {
        3
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Combined(combination) => Some(place(&Combination::ALL, &combination)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Combination::ALL.get(usize::from(variant))).map(|c| Kind::Combined(*c))
    }

    /// A larger group longer than the others.
    fn seconds(&self, kind: Kind) -> u16 {
        match kind {
            Kind::Combined(Combination::Enclosed | Combination::Cluster) => LARGER,
            _ => FACTS[0].seconds,
        }
    }

    /// A group's lines, given and compared as a single line's (combined lines are lines copied
    /// as one shape).
    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Combined(combination) = exercise.kind else {
            return held(Blank);
        };
        let group = combined::CombinedLines::generate(combination, exercise.seed);
        held(GivenLines(group.lines))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::combined())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let arrangements = match level {
            SkillLevel::First => FIRST.to_vec(),
            SkillLevel::Second => rotated(&SECOND, turn.turn, SECOND.len()),
            SkillLevel::Third => rotated(&THIRD, turn.turn, THIRD.len()),
        };
        laid(arrangements.into_iter().map(Kind::Combined))
    }

    /// Two lines joined, each gently bowed.
    fn glyph(&self) -> Vec<TileMark> {
        let bow = |t: f32| (std::f32::consts::PI * t).sin();
        vec![
            TileMark::Line(curve(|t| [9.0 + 15.0 * t, 30.0 - 19.0 * t - 2.0 * bow(t)])),
            TileMark::Line(curve(|t| [24.0 + 21.0 * t, 11.0 + 16.0 * t - 2.5 * bow(t)])),
        ]
    }
}
