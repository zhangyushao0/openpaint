//! Single lines (`Kind::Line`): lines from curvature profiles, and the How of lines, combined lines
//! and single shapes.

pub(crate) mod demo_drawn;
pub(crate) mod line;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::LINES;
use line::{LineExercise, LineTier};

pub(crate) struct Lines;

const FACTS: [Facts; 1] = [Facts::of(Family::Line, "line", Work::Lines)];

impl Described for Lines {
    fn code(&self) -> u8 {
        1
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Line(tier) => Some(place(&LineTier::ALL, &tier)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        LineTier::ALL
            .get(usize::from(variant))
            .map(|t| Kind::Line(*t))
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Line(tier) = exercise.kind else {
            return exercise.picture();
        };
        Original::Lines(vec![LineExercise::generate(tier, exercise.seed).points])
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::Lines(lines) => marks::givens(lines),
            _ => Vec::new(),
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_drawn::line())
    }

    /// A drill: stage 1's six lines.
    fn drill(&self, _family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        LINES.map(|tier| (Kind::Line(tier), None)).to_vec()
    }
}
