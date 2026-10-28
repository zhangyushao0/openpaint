//! Drills (S91): one kind practised on purpose, chosen from the practice options' tiles (V大
//! drills one skill at a time, week by week; the author's 专项训练, 2026-10-28). A drill is a sheet
//! of that kind only, about ten minutes and no warm-up: 6 single lines, 4 groups of combined lines
//! (each arrangement), 6 colours (stage 1's), 3 single shapes (their tiers in turn from entry to
//! hard, the last the user's own as in stage 2), 2 colour combinations, 2 value scales, 3 boxes, a
//! page of free boxes, 4 ellipses, 5 points, proportions or angles, or 3 of one of V大's five
//! tricks; each kind's variants in turn by the drill's number among its kind's drills. Its record
//! is one of stage `DRILL` (`record.rs`): it counts in no session, road or stage.

use crate::areas::Arrangement;
use crate::colour_match::Tone;
use crate::ellipse::Guide;
use crate::flat::Estimate;
use crate::pack::Tier;
use crate::perspective::BoxView;
use crate::plan::{Family, Kind};
use crate::scale::ScaleTone;
use crate::session::{Before, COLOURS, COMBINED, HORIZONS, LINES, Picker, Plan, seeded};
use crate::space::Trick;

/// The kinds each ready stage practises, as the practice options' tiles show them.
const STAGE_ONE: [Family; 3] = [Family::Line, Family::Combined, Family::Colour];
const STAGE_TWO: [Family; 6] = [
    Family::Shape,
    Family::Areas,
    Family::Scale,
    Family::Box,
    Family::FreeBoxes,
    Family::Ellipse,
];
const STAGE_THREE: [Family; 8] = [
    Family::Point,
    Family::Proportion,
    Family::Angle,
    Family::Centre,
    Family::Halves,
    Family::Thirds,
    Family::Step,
    Family::Mirror,
];

/// How many exercises a drill of each kind holds.
const SHAPES: usize = 3;
const COMBINATIONS: usize = 2;
const SCALES: usize = 2;
const BOXES: usize = 3;
const ELLIPSES: usize = 4;
const ESTIMATES: usize = 5;
const TRICKS: usize = 3;

/// The kinds `stage` practises, in its tiles' order; none for a stage not ready.
#[must_use]
pub const fn kinds_of(stage: u8) -> &'static [Family] {
    match stage {
        1 => &STAGE_ONE,
        2 => &STAGE_TWO,
        3 => &STAGE_THREE,
        _ => &[],
    }
}

/// Drill `number` (from 1) of `family`, seeded `seed`, after `before` (the pictures there are and
/// those shown before; nothing met is drawn on: a drill has no warm-up). None for a kind no stage
/// drills (a colour copy) and for shapes without any picture.
#[must_use]
pub fn drill(family: Family, number: u32, seed: u64, before: &Before<'_>) -> Option<Plan> {
    let turn = number.saturating_sub(1) as usize;
    let all = match family {
        Family::Shape => shapes(turn, seed, before),
        Family::Study => Vec::new(),
        _ => kinds(family, turn).into_iter().map(|k| (k, None)).collect(),
    };
    (!all.is_empty()).then(|| Plan {
        exercises: seeded(seed, all.into_iter()),
        warm_up: 0,
    })
}

/// `count` of `variants` from the `turn`th drill's place on, round and round.
fn in_turn<T: Copy>(variants: &[T], turn: usize, count: usize) -> Vec<T> {
    (0..count)
        .map(|k| variants[(count * turn + k) % variants.len()])
        .collect()
}

/// The kinds of a drill of `family` (not shapes) after `turn` others.
fn kinds(family: Family, turn: usize) -> Vec<Kind> {
    let estimate = Estimate::ALL.into_iter().find(|e| e.family() == family);
    let trick = Trick::ALL.into_iter().find(|t| t.family() == family);
    match (family, estimate, trick) {
        (_, Some(estimate), _) => vec![Kind::Flat(estimate); ESTIMATES],
        (_, _, Some(trick)) => vec![Kind::Space(trick); TRICKS],
        (Family::Line, ..) => LINES.map(Kind::Line).to_vec(),
        (Family::Combined, ..) => COMBINED.map(Kind::Combined).to_vec(),
        (Family::Colour, ..) => COLOURS
            .map(|(purity, lightness)| Kind::Colour(Tone { purity, lightness }))
            .to_vec(),
        (Family::Areas, ..) => in_turn(&Arrangement::ALL, turn, COMBINATIONS)
            .into_iter()
            .map(Kind::Areas)
            .collect(),
        (Family::Scale, ..) => (in_turn(&ScaleTone::ALL, turn, SCALES).into_iter())
            .map(Kind::Scale)
            .collect(),
        (Family::Box, ..) => (in_turn(&BoxView::ALL, turn, BOXES).into_iter())
            .map(Kind::Box)
            .collect(),
        (Family::FreeBoxes, ..) => in_turn(&HORIZONS, turn, 1)
            .into_iter()
            .map(Kind::FreeBoxes)
            .collect(),
        (Family::Ellipse, ..) => (in_turn(&Guide::ALL, turn, ELLIPSES).into_iter())
            .map(Kind::Ellipse)
            .collect(),
        _ => Vec::new(),
    }
}

/// A shapes drill's shapes after `turn` others, with their pictures: three tiers in turn from
/// entry to hard, easiest first, the last the user's own of the hardest or an easier one when they
/// have one (as stage 2's); fewer, or none, where no picture of a tier is left.
fn shapes(
    turn: usize,
    seed: u64,
    before: &Before<'_>,
) -> Vec<(Kind, Option<crate::pack::ItemRef>)> {
    let mut tiers = in_turn(&Tier::ALL, turn, SHAPES);
    tiers.sort();
    let mut picker = Picker::new(seed, before, tiers[tiers.len() - 1]);
    picker.shapes(&tiers)
}
