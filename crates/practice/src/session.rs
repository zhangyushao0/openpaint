//! Stage 1's session: a warm-up of about five minutes from the kinds already met, then the
//! stage's mix: six single lines (one near-straight, then bends of every kind), four groups of
//! combined lines (each arrangement once) and six colours (four low in saturation, every
//! lightness).
//!
//! Its length, after S75's estimates (a line about 0.8 minutes, a colour about 1.75) and a group
//! of combined lines taken at about 2: the mix about 23 minutes, the warm-up about 5, so a session
//! from the second on is about 28 and the first about 23. The warm-up's four exercises take the
//! families met in turn (lines, combined lines, colours), each a kind of its family already met,
//! chosen by the seed: two lines and two colours while those are all that was met (about 5
//! minutes), two lines, a group and a colour once combined lines were too (about 5.4). The mix
//! keeps a session's first colour where S75's sheet had it (the eleventh pair).

use crate::colour_match::{Lightness, Purity, Tone};
use crate::combined::Combination;
use crate::line::LineTier;
use crate::plan::{Exercise, Family, Kind};
use crate::random::{Random, derived};

/// Exercises a warm-up holds.
pub const WARM_UP: usize = 4;

/// The mix's lines, combined lines and colours, in order.
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

/// A session's exercises, the first `warm_up` of them its warm-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub exercises: Vec<Exercise>,
    pub warm_up: u32,
}

/// Stage 1's session seeded `seed` after the kinds `met` in earlier sessions: its warm-up from
/// them (none when nothing was met), then the stage's mix. Exercise `i`'s seed is
/// `derived(seed, i)`.
#[must_use]
pub fn stage_one(seed: u64, met: &[Kind]) -> Plan {
    let warm_up = warm_up(seed, met);
    let lines = LINES.map(Kind::Line);
    let combined = COMBINED.map(Kind::Combined);
    let colours = COLOURS.map(|(purity, lightness)| Kind::Colour(Tone { purity, lightness }));
    let exercises = (warm_up.iter().copied())
        .chain(lines)
        .chain(combined)
        .chain(colours)
        .enumerate()
        .map(|(index, kind)| Exercise {
            kind,
            seed: derived(seed, index as u64),
        })
        .collect();
    Plan {
        exercises,
        warm_up: u32::try_from(warm_up.len()).unwrap_or(0),
    }
}

/// The warm-up's kinds: `WARM_UP` of the kinds `met`, the families met taken in turn, each a
/// kind of its family chosen by the seed; then lines first, combined lines, colours.
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
