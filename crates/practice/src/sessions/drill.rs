//! Drills (S91): one kind practised on purpose, chosen from the practice options' tiles (V大
//! drills one skill at a time, week by week; the author's 专项训练, 2026-10-28). A drill is a sheet
//! of that kind only, about ten minutes and no warm-up: 6 single lines, 4 groups of combined lines
//! (each arrangement), 6 colours (stage 1's), 3 single shapes (their tiers in turn from entry to
//! hard, the last the user's own as in stage 2), 2 colour combinations (crops not shown before
//! while any are left, S101), 2 value scales, 3 boxes, a page of free boxes, 4 ellipses, 5 points,
//! proportions or angles, 3 of one of V大's five tricks, 3 of a solid (S93), 3 objects (S94, their
//! carves in turn), 3 divisions, 3 value masses (S96) or a whole copy (S95, compositions of the
//! simple and medium tiers in turn), 2 key shifts (S107, their leans in turn); each kind's
//! variants in turn by the drill's number among its kind's drills (a solid's pose by its seed).
//! Its record is one of stage `DRILL` (`record.rs`): it counts in no session, road or stage.

use crate::geometry::ellipse::Guide;
use crate::geometry::perspective::BoxView;
use crate::kinds::colours::colour_match::Tone;
use crate::kinds::estimates::flat::Estimate;
use crate::kinds::objects::object_design::Carve;
use crate::kinds::points_in_space::space::Trick;
use crate::kinds::scales::scale::ScaleTone;
use crate::kinds::solids::solid::Solid;
use crate::packs::pack::Tier;
use crate::sessions::plan::{Family, Kind};
use crate::sessions::session::{
    Before, COLOURS, COMBINED, FULL, HORIZONS, LINES, Picker, Plan, seeded,
};
use crate::sessions::session_five::key_shifts;
use crate::sessions::session_four::COMPOSITION_TIERS;

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
const STAGE_THREE: [Family; 11] = [
    Family::Point,
    Family::Proportion,
    Family::Angle,
    Family::Centre,
    Family::Halves,
    Family::Thirds,
    Family::Step,
    Family::Mirror,
    Family::Cylinder,
    Family::Cone,
    Family::Sphere,
];

/// Stage 4's kinds: objects simplified into boxes (S94), a picture divided (S95), its value masses
/// (S96) and one copied whole (S95).
const STAGE_FOUR: [Family; 4] = [
    Family::Object,
    Family::Division,
    Family::ValueMasses,
    Family::WholeCopy,
];

/// Stage 5's kinds: key shifts (S107); abstract analysis and structure rotation join them.
const STAGE_FIVE: [Family; 1] = [Family::KeyShift];

/// How many exercises a drill of each kind holds.
const SHAPES: usize = 3;
const COMBINATIONS: usize = 2;
const SCALES: usize = 2;
const BOXES: usize = 3;
const ELLIPSES: usize = 4;
const ESTIMATES: usize = 5;
const TRICKS: usize = 3;
const SOLIDS: usize = 3;
const OBJECTS: usize = 3;
const DIVISIONS: usize = 3;
const WHOLE_COPIES: usize = 1;
const VALUE_MASSES: usize = 3;
const KEY_SHIFTS: usize = 2;

/// The kinds `stage` practises, in its tiles' order; none for a stage not ready.
#[must_use]
pub const fn kinds_of(stage: u8) -> &'static [Family] {
    match stage {
        1 => &STAGE_ONE,
        2 => &STAGE_TWO,
        3 => &STAGE_THREE,
        4 => &STAGE_FOUR,
        5 => &STAGE_FIVE,
        _ => &[],
    }
}

/// Drill `number` (from 1) of `family`, seeded `seed`, after `before` (the pictures there are and
/// those shown before, the value scales met; no kind met is drawn on: a drill has no warm-up).
/// None for a kind no stage
/// drills (a colour copy), for shapes without any picture, for divisions and whole copies without
/// a composition, for value masses without one whose masses read and for combinations without a
/// crop.
#[must_use]
pub fn drill(family: Family, number: u32, seed: u64, before: &Before<'_>) -> Option<Plan> {
    let turn = number.saturating_sub(1) as usize;
    let all = match family {
        Family::Shape => shapes(turn, seed, before),
        Family::Division | Family::WholeCopy | Family::ValueMasses => {
            compositions(family, turn, seed, before)
        }
        Family::Areas => {
            let mut picker = Picker::new(seed, before, Tier::Simple);
            (0..COMBINATIONS).filter_map(|_| picker.crop()).collect()
        }
        Family::KeyShift => {
            let mut picker = Picker::new(seed, before, Tier::Simple);
            key_shifts(&mut picker, (KEY_SHIFTS, turn), (seed, before))
        }
        Family::Study => Vec::new(),
        _ => kinds(family, turn).into_iter().map(|k| (k, None)).collect(),
    };
    (!all.is_empty()).then(|| Plan {
        exercises: seeded(seed, all.into_iter(), before.scales),
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
    if let Some(solid) = Solid::ALL.into_iter().find(|s| s.family() == family) {
        return vec![Kind::Solid(solid); SOLIDS];
    }
    match (family, estimate, trick) {
        (_, Some(estimate), _) => vec![Kind::Flat(estimate); ESTIMATES],
        (_, _, Some(trick)) => vec![Kind::Space(trick); TRICKS],
        (Family::Line, ..) => LINES.map(Kind::Line).to_vec(),
        (Family::Combined, ..) => COMBINED.map(Kind::Combined).to_vec(),
        (Family::Colour, ..) => COLOURS
            .map(|(purity, lightness)| Kind::Colour(Tone { purity, lightness }))
            .to_vec(),
        (Family::Scale, ..) => (in_turn(&ScaleTone::ALL, turn, SCALES).into_iter())
            .map(|tone| Kind::Scale(tone, FULL))
            .collect(),
        (Family::Box, ..) => (in_turn(&BoxView::ALL, turn, BOXES).into_iter())
            .map(Kind::Box)
            .collect(),
        (Family::FreeBoxes, ..) => in_turn(&HORIZONS, turn, 1)
            .into_iter()
            .map(Kind::FreeBoxes)
            .collect(),
        (Family::Object, ..) => (in_turn(&Carve::ALL, turn, OBJECTS).into_iter())
            .map(Kind::Object)
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
) -> Vec<(Kind, Option<crate::packs::pack::ItemRef>)> {
    let mut tiers = in_turn(&Tier::ALL, turn, SHAPES);
    tiers.sort();
    let mut picker = Picker::new(seed, before, tiers[tiers.len() - 1]);
    picker.shapes(&tiers)
}

/// A drill of divisions (3), whole copies (1) or value masses (3) after `turn` others, with their
/// compositions: the simple and medium tiers in turn, none twice, none of the last ten sessions'
/// while others are left; none without a composition (whose masses read, for value masses).
fn compositions(
    family: Family,
    turn: usize,
    seed: u64,
    before: &Before<'_>,
) -> Vec<(Kind, Option<crate::packs::pack::ItemRef>)> {
    let (count, kind): (usize, fn(Tier) -> Kind) = match family {
        Family::Division => (DIVISIONS, Kind::Division),
        Family::ValueMasses => (VALUE_MASSES, Kind::ValueMasses),
        _ => (WHOLE_COPIES, Kind::WholeCopy),
    };
    let kinds: Vec<Kind> = (in_turn(&COMPOSITION_TIERS, turn, count).into_iter())
        .map(kind)
        .collect();
    Picker::new(seed, before, Tier::Medium).compositions(&kinds)
}
