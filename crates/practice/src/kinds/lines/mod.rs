//! Single lines (`Kind::Line`): lines from curvature profiles, their How, their tile, their
//! lessons, and how a line's or a group's copy is compared (laid over it, fitted for Aligned:
//! `marks.rs`).
//!
//! A lesson (a line about 48 seconds) holds six: the first level its first (a near-straight line,
//! arcs, S-curves, a hook), the second arcs, S-curves, a hook and two three-bend meanders (more
//! key points to place), the third meanders, a hook and three rippled lines (the big trend first,
//! then the ripples on it; S141, V大's W1 key points and W2 hierarchy).

pub(crate) mod demo;
pub(crate) mod line;
pub(crate) mod marks;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Counted, Described, Facts, Kept, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::tile::{TileMark, curve};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use line::{LineExercise, LineTier};
use marks::GivenLines;

pub(crate) struct Lines;

const FACTS: [Facts; 1] = [Facts::of("line", ToolSet::DRAWING, 48)
    .aligned()
    .kept(Kept::Line)
    .counted(Counted::Line)];
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
    LineTier::Meander,
    LineTier::Hook,
    LineTier::Wave,
    LineTier::Meander,
];
const THIRD: [LineTier; 6] = [
    LineTier::Meander,
    LineTier::Layered,
    LineTier::Hook,
    LineTier::Layered,
    LineTier::Meander,
    LineTier::Layered,
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

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Line(tier) = exercise.kind else {
            return held(Blank);
        };
        held(GivenLines(vec![
            LineExercise::generate(tier, exercise.seed).points,
        ]))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
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

    /// A line, gently waved.
    fn glyph(&self) -> Vec<TileMark> {
        let wave = |t: f32| (std::f32::consts::TAU * t).sin();
        vec![TileMark::Line(curve(|t| {
            [10.0 + 34.0 * t, 30.0 - 19.0 * t + 4.0 * wave(t)]
        }))]
    }
}
