//! Combined lines (`Kind::Combined`): two to four lines copied as one shape, in four arrangements;
//! given, compared and fitted as a single line's lines are (`lines/marks.rs`).
//!
//! A lesson (a group about 2 minutes) holds three: the first level joined, crossing and branched
//! lines (stage 1's first three), the second the branched and the near ones with a crossing, the
//! third near lines, whose gaps are the proportions to keep, twice with a crossing; the
//! arrangements in turn by the lesson.

pub(crate) mod combined;
pub(crate) mod demo;

use crate::course::picker::{laid, rotated};
use crate::demos::demo::Demo;
use crate::kinds::lines::marks as line_marks;
use crate::kinds::{Described, Facts, Kind, Original, SkillLevel, Tools, Turn, place};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, curve};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;
use combined::Combination;

pub(crate) struct CombinedLines;

const FACTS: [Facts; 1] = [Facts::of(Family::Combined, "combined", Tools::Drawing, 120).aligned()];
/// A lesson's groups.
const COUNT: usize = 3;
/// Each level's arrangements: the first's in order, the others' in turn from the lesson's place.
const FIRST: [Combination; 3] = [
    Combination::Joined,
    Combination::Crossing,
    Combination::Branched,
];
const SECOND: [Combination; 3] = [
    Combination::Branched,
    Combination::Near,
    Combination::Crossing,
];
const THIRD: [Combination; 3] = [Combination::Near, Combination::Crossing, Combination::Near];

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

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Combined(combination) = exercise.kind else {
            return exercise.picture();
        };
        Original::Lines(combined::CombinedLines::generate(combination, exercise.seed).lines)
    }

    /// A group's lines, as a single line's (combined lines are lines copied as one shape).
    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        original.lines().map(line_marks::givens).unwrap_or_default()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::combined())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let arrangements = match level {
            SkillLevel::First => FIRST.to_vec(),
            SkillLevel::Second => rotated(&SECOND, turn.turn, COUNT),
            SkillLevel::Third => rotated(&THIRD, turn.turn, COUNT),
        };
        laid(arrangements.into_iter().map(Kind::Combined))
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        line_marks::compare(asked)
    }

    /// Two lines joined, each gently bowed.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        let bow = |t: f32| (std::f32::consts::PI * t).sin();
        vec![
            TileMark::Line(curve(|t| [9.0 + 15.0 * t, 30.0 - 19.0 * t - 2.0 * bow(t)])),
            TileMark::Line(curve(|t| [24.0 + 21.0 * t, 11.0 + 16.0 * t - 2.5 * bow(t)])),
        ]
    }
}
