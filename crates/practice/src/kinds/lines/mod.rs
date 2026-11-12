//! Single lines (`Kind::Line`): lines from curvature profiles, their How, their tile, their
//! lessons, and how a line's or a group's copy is compared (laid over it, fitted for Aligned:
//! `marks.rs`).
//!
//! A lesson (a line about 48 seconds) holds six: the first level stage 1's (a near-straight line,
//! arcs, S-curves, a hook), the second arcs, S-curves and hooks in even shares, the third
//! S-curves and hooks alone, the bends hardest to keep.

pub(crate) mod demo;
pub(crate) mod line;
pub(crate) mod marks;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kept, Kind, Original, SkillLevel, Tools, Turn, place};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, curve};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;
use line::{LineExercise, LineTier};

pub(crate) struct Lines;

const FACTS: [Facts; 1] = [Facts::of(Family::Line, "line", Tools::Drawing, 48)
    .aligned()
    .kept(Kept::Line)];
/// Each level's lines in order.
const FIRST: [LineTier; 6] = [
    LineTier::Straight,
    LineTier::Arc,
    LineTier::Wave,
    LineTier::Arc,
    LineTier::Hook,
    LineTier::Wave,
];
const SECOND: [LineTier; 6] = [
    LineTier::Arc,
    LineTier::Wave,
    LineTier::Hook,
    LineTier::Arc,
    LineTier::Wave,
    LineTier::Hook,
];
const THIRD: [LineTier; 6] = [
    LineTier::Wave,
    LineTier::Hook,
    LineTier::Hook,
    LineTier::Wave,
    LineTier::Hook,
    LineTier::Wave,
];

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

    fn lesson(&self, level: SkillLevel, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let tiers: &[LineTier] = match level {
            SkillLevel::First => &FIRST,
            SkillLevel::Second => &SECOND,
            SkillLevel::Third => &THIRD,
        };
        laid(tiers.iter().copied().map(Kind::Line))
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
