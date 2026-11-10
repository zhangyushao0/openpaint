//! Drills (S91): one kind practised on purpose, chosen from the practice options' tiles (V大
//! drills one skill at a time, week by week; the author's 专项训练, 2026-10-28). A drill is a sheet
//! of that kind only, about ten minutes and no warm-up; what it holds is its kind's description's
//! (`Described::drill`, each kind's variants in turn by the drill's number among its kind's
//! drills). Its sheet's lesson is one of a kind chosen (`Picked::Chosen`, `sheet.rs`): it counts in
//! no session, road or stage.

use crate::kinds::{Needs, Turn};
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::plan::{Family, Kind};
use crate::sessions::session::{Before, Picker, Plan, seeded};
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

/// What `stage`'s sessions need kept before one starts: stage 5's key shifts a crop (S107).
#[must_use]
pub const fn stage_needs(stage: u8) -> Needs {
    match stage {
        5 => Needs::Crops,
        _ => Needs::Nothing,
    }
}

/// Drill `number` (from 1) of `family`, seeded `seed`, after `before` (the pictures there are and
/// those shown before, the value scales met; no kind met is drawn on: a drill has no warm-up).
/// None for a kind no stage drills (a colour copy), and where its kind finds no picture to drill
/// on (shapes, compositions, crops).
#[must_use]
pub fn drill(family: Family, number: u32, seed: u64, before: &Before<'_>) -> Option<Plan> {
    let turn = Turn {
        turn: number.saturating_sub(1) as usize,
        seed,
        before,
    };
    let all = family.described().drill(family, &turn);
    (!all.is_empty()).then(|| Plan {
        exercises: seeded(seed, all.into_iter(), before.scales),
        warm_up: 0,
    })
}

/// `count` of `variants` from the `turn`th drill's place on, round and round.
pub(crate) fn in_turn<T: Copy>(variants: &[T], turn: usize, count: usize) -> Vec<T> {
    (0..count)
        .map(|k| variants[(count * turn + k) % variants.len()])
        .collect()
}

/// `count` kinds made by `kind` of `variants` in turn, no pictures.
pub(crate) fn turned<T: Copy>(
    (variants, count): (&[T], usize),
    turn: &Turn<'_>,
    kind: impl Fn(T) -> Kind,
) -> Vec<(Kind, Option<ItemRef>)> {
    (in_turn(variants, turn.turn, count).into_iter())
        .map(|variant| (kind(variant), None))
        .collect()
}

/// A drill of `count` compositions of the kind `kind` makes: the simple and medium tiers in turn,
/// none twice, none of the last ten sessions' while others are left; none without a composition
/// (whose masses read, for value masses).
pub(crate) fn compositions(
    count: usize,
    turn: &Turn<'_>,
    kind: fn(Tier) -> Kind,
) -> Vec<(Kind, Option<ItemRef>)> {
    let kinds: Vec<Kind> = (in_turn(&COMPOSITION_TIERS, turn.turn, count).into_iter())
        .map(kind)
        .collect();
    Picker::new(turn.seed, turn.before, Tier::Medium).compositions(&kinds)
}
