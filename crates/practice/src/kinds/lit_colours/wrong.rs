//! Copies of colour under light made wrong on purpose (S154j), one a rule, every region but those it
//! changes the model's: what its contact sheet (`openpaint-harness lit-colours`) and its tests
//! paint to show each rule marks what it should, each colour dimmed or brightened as light scales
//! it. The design's four (S154): the light family the local colour; the shadow family the local
//! darkened only (at 稳 its reflected light too, which then lacks the bounce as well); the light
//! family the light's colour only; the ground's bounce left out. And two for lightness: the shadow
//! nearly as light as the light (it breaks the value too where the model's families lie further
//! apart than half a step and two values' bands), the lit ground a step too light.

use crate::kinds::lit_colours::check::{DARKER, HUE_KEPT, Rule, VALUE_OFF, dimmed};
use crate::kinds::lit_colours::regions::Region;
use crate::kinds::lit_colours::scene::ColourTask;
use crate::maths::colour::{Rgb8, from_oklab, oklab, oklch};

/// How far a copy made too light lies off the model (Oklab `L`).
const TOO_LIGHT: f64 = 0.2;

impl ColourTask {
    /// The copy made wrong on purpose for `rule`, as the regions it changes and their colours;
    /// none for a rule the level does not hold, or where its wrong copy would be right (the light's
    /// colour within the local colour's hue).
    #[must_use]
    pub fn wrong(&self, rule: Rule) -> Option<Vec<(Region, Rgb8)>> {
        if !self.rules().contains(&rule) {
            return None;
        }
        // `region` painted `of` dimmed or brightened to the model's lightness there, `by` lighter.
        let painted = |region: Region, of: Rgb8, by: f64| -> Option<(Region, Rgb8)> {
            let [l, ..] = oklab(self.model(region)?);
            let l = (l + by).clamp(0.0, 1.0);
            let [a, b] = dimmed(of, l);
            Some((region, from_oklab([l, a, b])))
        };
        let local = self.colours.local;
        Some(match rule {
            Rule::Darker => {
                let lightness = |region| self.model(region).map(|c| oklab(c)[0]);
                let gap = lightness(Region::Light)? - lightness(Region::Shadow)?;
                // The families brought within half the step, each moved at most a value's band.
                let closed = (gap - DARKER / 2.0).max(0.0);
                let each = (closed / 2.0).min(VALUE_OFF - 0.01);
                let rest = (closed - each).max(0.0);
                vec![
                    painted(Region::Light, self.model(Region::Light)?, -each)?,
                    painted(Region::Shadow, self.model(Region::Shadow)?, rest)?,
                ]
            }
            Rule::LightColour => vec![painted(Region::Light, local, 0.0)?],
            Rule::ShadowColour => {
                let family = [Region::Shadow, Region::Reflected].into_iter();
                (family.filter(|region| self.asked().contains(region)))
                    .map(|region| painted(region, local, 0.0))
                    .collect::<Option<Vec<_>>>()?
            }
            Rule::LocalKept => {
                let [_, _, light] = oklch(self.colours.light);
                let [_, _, own] = oklch(local);
                let turned = ((light - own + 180.0).rem_euclid(360.0) - 180.0).abs();
                if turned <= HUE_KEPT + 15.0 {
                    return None;
                }
                vec![painted(Region::Light, self.colours.light, 0.0)?]
            }
            Rule::Bounce => vec![painted(
                Region::Reflected,
                self.model(Region::Shadow)?,
                0.0,
            )?],
            Rule::Value => {
                let ground = self.model(Region::Ground)?;
                vec![painted(Region::Ground, ground, TOO_LIGHT)?]
            }
        })
    }
}
