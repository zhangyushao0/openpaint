//! Combined lines (`Kind::Combined`): two to four lines copied as one shape, in four arrangements.

pub(crate) mod combined;

use crate::demos::demo::Demo;
use crate::kinds::lines::demo_drawn;
use crate::kinds::lines::marks as line_marks;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::COMBINED;
use combined::Combination;

pub(crate) struct CombinedLines;

const FACTS: [Facts; 1] = [Facts::of(Family::Combined, "combined", Work::Lines)];

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
        match original {
            Original::Lines(lines) => line_marks::givens(lines),
            _ => Vec::new(),
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_drawn::combined())
    }

    /// A drill: each arrangement once.
    fn drill(&self, _family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        COMBINED.map(|c| (Kind::Combined(c), None)).to_vec()
    }
}
