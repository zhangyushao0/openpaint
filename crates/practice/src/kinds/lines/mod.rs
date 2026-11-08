//! Single lines (`Kind::Line`): lines from curvature profiles, their How, their tile, and how a
//! line's or a group's copy is compared (laid over it, fitted for Aligned: `marks.rs`).

pub(crate) mod demo;
pub(crate) mod line;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kept, Kind, Original, Turn, Work, place};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, curve};
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::LINES;
use line::{LineExercise, LineTier};

pub(crate) struct Lines;

const FACTS: [Facts; 1] = [Facts::of(Family::Line, "line", Work::Lines).kept(Kept::Line)];

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
        original.lines().map(marks::givens).unwrap_or_default()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::line())
    }

    /// A drill: stage 1's six lines.
    fn drill(&self, _family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        LINES.map(|tier| (Kind::Line(tier), None)).to_vec()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    /// A line, gently waved.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        let wave = |t: f32| (std::f32::consts::TAU * t).sin();
        vec![TileMark::Line(curve(|t| {
            [10.0 + 34.0 * t, 30.0 - 19.0 * t + 4.0 * wave(t)]
        }))]
    }
}
