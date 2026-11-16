//! The mix the sheet's tests lay: six lines, four combined lines and six colours.

use openpaint_practice::{Combination, Exercise, Kind, Lightness, LineTier, Purity, Tone, derived};

/// Six lines, four combined lines and six colours, exercise `i` seeded `derived(seed, i)`.
pub fn mixed(seed: u64) -> Vec<Exercise> {
    let lines = [
        LineTier::Straight,
        LineTier::Arc,
        LineTier::Wave,
        LineTier::Arc,
        LineTier::Hook,
        LineTier::Wave,
    ]
    .map(Kind::Line);
    let combined = [
        Combination::Joined,
        Combination::Crossing,
        Combination::Branched,
        Combination::Near,
    ]
    .map(Kind::Combined);
    let tone = |purity, lightness| Kind::Colour(Tone { purity, lightness });
    let colours = [
        tone(Purity::Muted, Lightness::Middle),
        tone(Purity::Grey, Lightness::Light),
        tone(Purity::Middle, Lightness::Dark),
        tone(Purity::Muted, Lightness::Light),
        tone(Purity::Vivid, Lightness::Middle),
        tone(Purity::Grey, Lightness::Dark),
    ];
    (lines.into_iter().chain(combined).chain(colours))
        .enumerate()
        .map(|(at, kind)| Exercise::generated(kind, derived(seed, at as u64)))
        .collect()
}
