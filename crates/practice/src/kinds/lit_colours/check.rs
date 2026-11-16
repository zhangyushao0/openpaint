//! Colour under light's rules (S154j), each read directly on the copy's regions (`regions.rs`)
//! against the colours the exercise gave and the model's (`render.rs`), in Oklab:
//!
//! 1. Light and shadow (every level): the object's shadow family darker than its light family by
//!    `DARKER` at least, the cast shadow darker than the lit ground alike.
//! 2. The light's colour (every level): the light family's `a`, `b` moved from the local colour's
//!    toward the light's by `MOVED` (key shifts' just-noticeable step) at least.
//! 3. The shadow's colour (every level): the shadow family moved toward the sky's colour from the
//!    local's and from the light family's by `MOVED`, the cast shadow from the lit ground's.
//! 4. The local colour kept (熟练, 稳): both families' hue within `HUE_KEPT` of the local's, their
//!    chroma `CHROMA_KEPT` of the model's at least.
//! 5. The surroundings (稳): the reflected light moved from the shadow toward the ground's colour
//!    by `MOVED`.
//! 6. Value kept (every level): each region's lightness within `VALUE_OFF` of the model's.
//!
//! A move is read against the colour it starts from merely dimmed or brightened to the lightness
//! it reaches (light scaled, its hue and its purity relative to its lightness kept): a shadow that
//! is the local colour darkened has moved nowhere, whatever the hues. A colour already as near its
//! target as a step, or nearer, need come only `NEAR_SHARE` of its way, as a key shift's.
//!
//! The rules are a model's (Gurney: sunlit shadows take the sky often, not always; a painter may
//! warm a shadow), so a region breaking one is marked and the learner's eye judges (D123).

use crate::kinds::key_shifts::key_shift_check::{AT, MOVED, NEAR_SHARE};
use crate::kinds::lit_colours::regions::Region;
use crate::kinds::lit_colours::scene::ColourTask;
use crate::maths::colour::{Rgb8, oklab, oklch};
use crate::maths::path::Point;

/// The shadow darker than the light by this at least (Oklab `L`): more than half a scene's step.
pub const DARKER: f64 = 0.12;
/// The local colour's hue kept within this (degrees), its chroma this share of the model's.
pub const HUE_KEPT: f64 = 60.0;
pub const CHROMA_KEPT: f64 = 0.4;
/// How much nearer the local colour's hue the model's own families keep (degrees, the scene's
/// slack).
const HUE_ROOM: f64 = 10.0;
/// A region's lightness within this of the model's (Oklab `L`).
pub const VALUE_OFF: f64 = 0.12;

/// A rule a region's colour keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rule {
    Darker,
    LightColour,
    ShadowColour,
    LocalKept,
    Bounce,
    Value,
}

impl Rule {
    pub const ALL: [Self; 6] = [
        Self::Darker,
        Self::LightColour,
        Self::ShadowColour,
        Self::LocalKept,
        Self::Bounce,
        Self::Value,
    ];

    /// Whether it is about lightness alone (what the Grey view shows).
    #[must_use]
    pub const fn lightness(self) -> bool {
        matches!(self, Self::Darker | Self::Value)
    }
}

/// `colour`'s place in Oklab's `a`, `b` plane.
fn ab(colour: Rgb8) -> Point {
    let [_, a, b] = oklab(colour);
    [a, b]
}

/// `colour` dimmed or brightened to the lightness `l` as light scales it (Oklab's three parts
/// scale alike), its place in the `a`, `b` plane.
#[must_use]
pub fn dimmed(colour: Rgb8, l: f64) -> Point {
    let [l0, a, b] = oklab(colour);
    let k = l / l0.max(1e-6);
    [a * k, b * k]
}

/// Whether `to` moved from `from` toward `target` far enough (Oklab `a`, `b`), as a key shift's
/// area toward its tendency: the larger of its move along `target`'s own direction (its hue) and
/// how much nearer `target` it came, `MOVED` at least, or `NEAR_SHARE` of the way where `from`
/// lies nearer, `slack` times that; at `target` already, nothing.
#[must_use]
pub fn moved(from: Point, to: Point, target: Point, slack: f64) -> bool {
    let long = (target[0] - from[0]).hypot(target[1] - from[1]);
    if long <= AT {
        return true;
    }
    let reach = target[0].hypot(target[1]).max(f64::MIN_POSITIVE);
    let along = ((to[0] - from[0]) * target[0] + (to[1] - from[1]) * target[1]) / reach;
    let nearer = long - (target[0] - to[0]).hypot(target[1] - to[1]);
    along.max(nearer) >= slack * MOVED.min(NEAR_SHARE * long)
}

impl ColourTask {
    /// The rules `copy` (each region asked with its colour, none where not painted) breaks, each
    /// with the region it marks; a rule needing a region not painted is not read.
    #[must_use]
    pub fn broken(&self, copy: &[(Region, Option<Rgb8>)]) -> Vec<(Rule, Region)> {
        self.broken_by(copy, 1.0)
    }

    /// The rules `copy` breaks held `slack` times as hard on its moves and its hue (the scene's
    /// rule holds the model's own colours so, with room to spare).
    pub(crate) fn broken_by(
        &self,
        copy: &[(Region, Option<Rgb8>)],
        slack: f64,
    ) -> Vec<(Rule, Region)> {
        let of = |region: Region| {
            copy.iter()
                .find(|(r, _)| *r == region)
                .and_then(|(_, c)| *c)
        };
        let rules = self.rules();
        let mut findings = lightness_findings(&of);
        findings.extend(self.colour_findings(&of, slack));
        for (region, colour) in copy {
            let model = self.model(*region);
            let kept = colour
                .zip(model)
                .map(|(mine, model)| (oklab(mine)[0] - oklab(model)[0]).abs() <= VALUE_OFF);
            findings.push((Rule::Value, *region, kept));
        }
        (findings.into_iter())
            .filter(|(rule, _, kept)| rules.contains(rule) && *kept == Some(false))
            .map(|(rule, region, _)| (rule, region))
            .collect()
    }

    /// Whether the colours of `of` moved as the light colours them (rules 2 to 5): each finding
    /// its rule, the region it marks and whether it is kept (none where not painted).
    fn colour_findings(
        &self,
        of: &dyn Fn(Region) -> Option<Rgb8>,
        slack: f64,
    ) -> Vec<(Rule, Region, Option<bool>)> {
        let local = self.colours.local;
        let [light, sky, ground] = {
            let c = &self.colours;
            [c.light, c.sky, c.ground].map(ab)
        };
        // `region` moved toward `target` from `from` dimmed to its lightness.
        let towards = |from: Option<Rgb8>, region: Region, target: Point| {
            let to = of(region)?;
            Some(moved(dimmed(from?, oklab(to)[0]), ab(to), target, slack))
        };
        let shadow =
            [Some(local), of(Region::Light)].map(|from| towards(from, Region::Shadow, sky));
        let both = (shadow.iter()).try_fold(true, |all, kept| Some(all && (*kept)?));
        let mut findings = vec![
            (
                Rule::LightColour,
                Region::Light,
                towards(Some(local), Region::Light, light),
            ),
            (Rule::ShadowColour, Region::Shadow, both),
            (
                Rule::ShadowColour,
                Region::Cast,
                towards(of(Region::Ground), Region::Cast, sky),
            ),
            (
                Rule::Bounce,
                Region::Reflected,
                towards(of(Region::Shadow), Region::Reflected, ground),
            ),
        ];
        for region in [Region::Light, Region::Shadow] {
            let kept = self.local_kept(region, of(region), slack);
            findings.push((Rule::LocalKept, region, kept));
        }
        findings
    }

    /// Whether `colour` of `region` keeps the local colour: its hue within `HUE_KEPT` of the
    /// local's, its chroma `CHROMA_KEPT` of the model's at least; none where not painted.
    fn local_kept(&self, region: Region, colour: Option<Rgb8>, slack: f64) -> Option<bool> {
        let [_, chroma, hue] = oklch(colour?);
        let [_, model, _] = oklch(self.model(region)?);
        let [_, _, local] = oklch(self.colours.local);
        let turned = ((hue - local + 180.0).rem_euclid(360.0) - 180.0).abs();
        let kept = if slack > 1.0 {
            HUE_KEPT - HUE_ROOM
        } else {
            HUE_KEPT
        };
        Some(turned <= kept && chroma >= CHROMA_KEPT * model)
    }
}

/// Whether the shadows of `of` are darker than their lights (rule 1): the object's shadow family
/// than its light family, the cast shadow than the lit ground.
fn lightness_findings(of: &dyn Fn(Region) -> Option<Rgb8>) -> Vec<(Rule, Region, Option<bool>)> {
    let darker =
        |dark: Region, light: Region| Some(oklab(of(dark)?)[0] <= oklab(of(light)?)[0] - DARKER);
    vec![
        (
            Rule::Darker,
            Region::Shadow,
            darker(Region::Shadow, Region::Light),
        ),
        (
            Rule::Darker,
            Region::Cast,
            darker(Region::Cast, Region::Ground),
        ),
    ]
}
