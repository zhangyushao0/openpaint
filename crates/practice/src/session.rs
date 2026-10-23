//! The stages' sessions, each a warm-up of about five minutes from the kinds already met, then the
//! stage's mix.
//!
//! Stage 1 (V大's week 1): six single lines (one near-straight, then bends of every kind), four
//! groups of combined lines (each arrangement once) and six colours (four low in saturation, every
//! lightness). After S75's estimates (a line about 0.8 minutes, a colour about 1.75) and a group of
//! combined lines taken at about 2: the mix about 23 minutes, the warm-up about 5, so a session
//! from the second on is about 28 and the first about 23. The mix keeps a session's first colour
//! where S75's sheet had it (the eleventh pair).
//!
//! Stage 2 (V大's week 2, single shapes the main exercise): four single shapes, then one group of
//! combined lines and two colours, stage 1's judgements kept alive while week 2's other exercises
//! wait for their slices. The shapes' tiers follow the session's number: entry and simple first,
//! medium from the third session, hard from the sixth; the user's own pictures, once there are
//! some, take the last place. A shape taken at about 3.5 minutes (a line drawing, basically
//! right, compared and corrected once): the mix about 19.5 minutes, so a session about 25 with a
//! warm-up of lines, a group and a colour, about 27.5 once shapes are met and one warms up too.
//!
//! The warm-up's four exercises take the families met in turn (lines, combined lines, shapes,
//! colours), each a kind of its family already met, chosen by the seed; a shape in it a picture of
//! its tier. A plan's shapes carry their pictures, picked by the seed (`pictures.rs`).

use std::collections::HashSet;

use crate::colour_match::{Lightness, Purity, Tone};
use crate::combined::Combination;
use crate::line::LineTier;
use crate::pack::{ItemRef, Tier};
use crate::pictures::Pictures;
use crate::plan::{Exercise, Family, Kind};
use crate::random::{Random, derived};

/// Exercises a warm-up holds.
pub const WARM_UP: usize = 4;

/// Stage 1's mix: lines, combined lines and colours, in order.
const LINES: [LineTier; 6] = [
    LineTier::Straight,
    LineTier::Arc,
    LineTier::Wave,
    LineTier::Arc,
    LineTier::Hook,
    LineTier::Wave,
];
const COMBINED: [Combination; 4] = [
    Combination::Joined,
    Combination::Crossing,
    Combination::Branched,
    Combination::Near,
];
const COLOURS: [(Purity, Lightness); 6] = [
    (Purity::Muted, Lightness::Middle),
    (Purity::Grey, Lightness::Light),
    (Purity::Middle, Lightness::Dark),
    (Purity::Muted, Lightness::Light),
    (Purity::Vivid, Lightness::Middle),
    (Purity::Grey, Lightness::Dark),
];
/// Stage 2's colours a session.
const STAGE_TWO_COLOURS: usize = 2;

/// A session's exercises, the first `warm_up` of them its warm-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub exercises: Vec<Exercise>,
    pub warm_up: u32,
}

/// What a session's plan reads: the kinds `met` in earlier sessions, the `pictures` there are and
/// those `used` in earlier sessions.
#[derive(Debug, Clone, Copy)]
pub struct Before<'a> {
    pub met: &'a [Kind],
    pub pictures: &'a Pictures,
    pub used: &'a [ItemRef],
}

/// Session `number` of `stage` seeded `seed` after `before`: stage 2's for stage 2, else stage
/// 1's.
#[must_use]
pub fn plan(stage: u8, number: u32, seed: u64, before: &Before<'_>) -> Plan {
    if stage == 2 {
        stage_two(number, seed, before)
    } else {
        stage_one(seed, before.met)
    }
}

/// Stage 1's session seeded `seed` after the kinds `met` in earlier sessions: its warm-up from
/// them (none when nothing was met), then the stage's mix. Exercise `i`'s seed is
/// `derived(seed, i)`; a shape met warms up with no picture (stage 1 has none).
#[must_use]
pub fn stage_one(seed: u64, met: &[Kind]) -> Plan {
    let met: Vec<Kind> = met
        .iter()
        .copied()
        .filter(|k| k.family() != Family::Shape)
        .collect();
    let warm_up = warm_up(seed, &met);
    let lines = LINES.map(Kind::Line);
    let combined = COMBINED.map(Kind::Combined);
    let colours = COLOURS.map(|(purity, lightness)| Kind::Colour(Tone { purity, lightness }));
    let kinds: Vec<Kind> = (warm_up.iter().copied())
        .chain(lines)
        .chain(combined)
        .chain(colours)
        .collect();
    Plan {
        exercises: seeded(seed, kinds.into_iter().map(|kind| (kind, None))),
        warm_up: u32::try_from(warm_up.len()).unwrap_or(0),
    }
}

/// Stage 2's session `number` seeded `seed` after `before`: its warm-up, then four shapes of the
/// session's tiers (the last the user's own when they have some), a group of combined lines and
/// two colours. Without any picture there are no shapes.
#[must_use]
pub fn stage_two(number: u32, seed: u64, before: &Before<'_>) -> Plan {
    let mut random = Random::new(derived(seed, u64::MAX - 1));
    let mut used: HashSet<ItemRef> = before.used.iter().cloned().collect();
    let mut pick = |kind: Kind, own: bool| -> Option<(Kind, Option<ItemRef>)> {
        let Kind::Shape(tier) = kind else {
            return Some((kind, None));
        };
        let personal = own.then(|| before.pictures.pick_personal(&mut random, &used));
        let (tier, item) = match personal.flatten() {
            Some(item) => (crate::pack::PERSONAL_TIER, item),
            None => before.pictures.pick(tier, &mut random, &used)?,
        };
        used.insert(item.clone());
        Some((Kind::Shape(tier), Some(item)))
    };
    let warm_up: Vec<(Kind, Option<ItemRef>)> = warm_up(seed, before.met)
        .into_iter()
        .filter_map(|kind| pick(kind, false))
        .collect();
    let tiers = shape_tiers(number);
    let own = before.pictures.has_personal();
    let shapes = (tiers.iter().enumerate())
        .filter_map(|(at, tier)| pick(Kind::Shape(*tier), own && at + 1 == tiers.len()));
    let turn = number.saturating_sub(1) as usize;
    let combined = Kind::Combined(COMBINED[turn % COMBINED.len()]);
    let colours = (0..STAGE_TWO_COLOURS).map(|k| {
        let (purity, lightness) = COLOURS[(turn * STAGE_TWO_COLOURS + k) % COLOURS.len()];
        Kind::Colour(Tone { purity, lightness })
    });
    let warmed = u32::try_from(warm_up.len()).unwrap_or(0);
    let all = (warm_up.into_iter())
        .chain(shapes.collect::<Vec<_>>())
        .chain([(combined, None)])
        .chain(colours.map(|kind| (kind, None)));
    Plan {
        exercises: seeded(seed, all),
        warm_up: warmed,
    }
}

/// The four shapes' tiers of stage 2's session `number`.
const fn shape_tiers(number: u32) -> [Tier; 4] {
    match number {
        0..=2 => [Tier::Entry, Tier::Entry, Tier::Simple, Tier::Simple],
        3..=5 => [Tier::Entry, Tier::Simple, Tier::Simple, Tier::Medium],
        6..=9 => [Tier::Simple, Tier::Simple, Tier::Medium, Tier::Hard],
        _ => [Tier::Simple, Tier::Medium, Tier::Medium, Tier::Hard],
    }
}

/// `kinds` (with their pictures) as exercises, exercise `i` seeded `derived(seed, i)`.
fn seeded(seed: u64, kinds: impl Iterator<Item = (Kind, Option<ItemRef>)>) -> Vec<Exercise> {
    kinds
        .enumerate()
        .map(|(index, (kind, picture))| Exercise {
            kind,
            seed: derived(seed, index as u64),
            picture,
        })
        .collect()
}

/// The warm-up's kinds: `WARM_UP` of the kinds `met`, the families met taken in turn, each a
/// kind of its family chosen by the seed; then in the families' order.
fn warm_up(seed: u64, met: &[Kind]) -> Vec<Kind> {
    let mut met = met.to_vec();
    met.sort_by_key(|kind| kind.code());
    met.dedup();
    let families: Vec<Family> = (Family::ALL.into_iter())
        .filter(|family| met.iter().any(|kind| kind.family() == *family))
        .collect();
    if families.is_empty() {
        return Vec::new();
    }
    let mut random = Random::new(derived(seed, u64::MAX));
    let mut kinds: Vec<Kind> = (0..WARM_UP)
        .filter_map(|turn| {
            let family = families[turn % families.len()];
            let of: Vec<&Kind> = met.iter().filter(|k| k.family() == family).collect();
            let at = (random.unit() * of.len() as f64) as usize;
            of.get(at.min(of.len().saturating_sub(1)))
                .map(|kind| **kind)
        })
        .collect();
    kinds.sort_by_key(|kind| kind.family());
    kinds
}
