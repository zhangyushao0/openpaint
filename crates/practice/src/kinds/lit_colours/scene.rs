//! Colour under light's scene (S154j): light on form's (`light_on_form/scene.rs`, the same
//! generator and its exact families) in colour. A form of unit 3 stands on the ground, lit from
//! three-quarter front by a light the pin gives; its parts are an object of a local colour, a ground
//! of its own, a light's colour and a sky's (`colours.rs`, by the level's `Palette`).
//!
//! A scene is laid from its seed's sequence until each region the level reads holds a readable
//! inner part; then coloured, again and again, until the model keeps every rule: its families at
//! least `FAMILIES_APART` apart in lightness, on the form and on the ground, and its own colours
//! breaking no rule, held `SLACK` times as hard on their moves and hues (`check.rs`), so a copy
//! painted near the model keeps them too. A colouring is tried on the model's colour at a few of
//! each region's cells, and the one kept read once from the model's render, as a copy is.

use crate::kinds::light_on_form::scene::{Form, LightTask, Lighting, Lit, Tones};
use crate::kinds::lit_colours::check::Rule;
use crate::kinds::lit_colours::colours::{Colours, coloured};
use crate::kinds::lit_colours::regions::{Region, Regions};
use crate::maths::colour::{Rgb8, oklab};
use crate::maths::random::{Random, derived};

/// Scenes laid, and colourings tried a scene, before the generator keeps the last.
const SCENES: u64 = 8;
const COLOURINGS: usize = 12;
/// The model's light and shadow families at least this far apart in lightness (Oklab `L`).
const FAMILIES_APART: f64 = 0.2;
/// How much harder the rules hold the model's own colours.
const SLACK: f64 = 1.25;
/// And on its colours as its render reads them, which a copy painted as the render is read near.
const READ_SLACK: f64 = 1.1;

/// The colours a level lights its scene with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Palette {
    /// A white or light grey object on a grey ground under a warm sun and a blue sky (入门).
    WarmSun,
    /// The same under a cool light and a warm sky (入门, in turn with the warm sun).
    CoolLight,
    /// A coloured object, its hue any of the wheel's twelve steps, under a warm sun and a blue sky
    /// (熟练).
    Local,
    /// A coloured lamp of any hue and a coloured ground bouncing into the shadow (稳).
    Bounce,
}

/// Colour under light's variant: the form and the level's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LitColour {
    pub form: Form,
    pub palette: Palette,
}

impl LitColour {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 11] = [
        Self::of(Form::Box, Palette::WarmSun),
        Self::of(Form::Cylinder, Palette::WarmSun),
        Self::of(Form::Sphere, Palette::WarmSun),
        Self::of(Form::Box, Palette::CoolLight),
        Self::of(Form::Cylinder, Palette::CoolLight),
        Self::of(Form::Sphere, Palette::CoolLight),
        Self::of(Form::Box, Palette::Local),
        Self::of(Form::Cylinder, Palette::Local),
        Self::of(Form::Cone, Palette::Local),
        Self::of(Form::Sphere, Palette::Local),
        Self::of(Form::Sphere, Palette::Bounce),
    ];

    const fn of(form: Form, palette: Palette) -> Self {
        Self { form, palette }
    }

    /// The light on form it is laid as: lit from three-quarter front, at 稳 its shadow family told
    /// apart (the reflected light).
    const fn lit(self) -> Lit {
        let tones = match self.palette {
            Palette::Bounce => Tones::Four,
            _ => Tones::Two,
        };
        Lit {
            form: self.form,
            lighting: Lighting::Front,
            tones,
        }
    }
}

/// A form in coloured light to paint, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourTask {
    pub variant: LitColour,
    pub task: LightTask,
    pub colours: Colours,
    pub(crate) regions: Regions,
}

impl ColourTask {
    /// The task `variant` and `seed` make.
    #[must_use]
    pub fn generate(variant: LitColour, seed: u64) -> Self {
        let mut random = Random::new(derived(seed, u64::MAX - 29));
        let bounce = variant.palette == Palette::Bounce;
        let mut last = None;
        for k in 0..SCENES {
            let task = LightTask::generate(variant.lit(), derived(seed, k));
            let regions = Regions::of(&task, bounce);
            let readable = |r: Region| regions.readable(r);
            if !(asked(bounce).into_iter()).all(readable) {
                continue;
            }
            for _ in 0..COLOURINGS {
                let mut made = Self {
                    variant,
                    task: task.clone(),
                    colours: coloured(variant.palette, &mut random),
                    regions: regions.clone(),
                };
                made.regions.colours = made.sampled_colours();
                if made.keeps(SLACK) {
                    made.regions.colours = made.model_colours();
                    if made.keeps(READ_SLACK) {
                        return made;
                    }
                }
                last = Some(made);
            }
        }
        let mut made = last.unwrap_or_else(|| Self::scene(variant, seed, &mut random));
        made.regions.colours = made.model_colours();
        made
    }

    /// The scene of `variant` its `seed` lays first, coloured once: what is kept where no
    /// colouring keeps the rules (never met by a seed tested).
    fn scene(variant: LitColour, seed: u64, random: &mut Random) -> Self {
        let task = LightTask::generate(variant.lit(), seed);
        let regions = Regions::of(&task, variant.palette == Palette::Bounce);
        Self {
            variant,
            task,
            colours: coloured(variant.palette, random),
            regions,
        }
    }

    /// The regions the level reads: the families on the form and on the ground, at 稳 the
    /// reflected light too.
    #[must_use]
    pub fn asked(&self) -> Vec<Region> {
        asked(self.bounces())
    }

    /// Whether the model keeps every rule held `slack` times as hard: each region asked held, the
    /// families apart, its colours breaking no rule.
    pub(crate) fn keeps(&self, slack: f64) -> bool {
        let held = self.asked().iter().all(|r| self.regions.readable(*r));
        let lightness = |region| self.model(region).map_or(0.0, |c| oklab(c)[0]);
        let apart = |light, shadow| lightness(light) - lightness(shadow) >= FAMILIES_APART;
        let model: Vec<(Region, Option<Rgb8>)> = (self.asked().into_iter())
            .map(|region| (region, self.model(region)))
            .collect();
        held && apart(Region::Light, Region::Shadow)
            && apart(Region::Ground, Region::Cast)
            && self.broken_by(&model, slack).is_empty()
    }

    /// The model's colour of `region` (as its render reads); none where the scene holds no such
    /// region.
    #[must_use]
    pub fn model(&self, region: Region) -> Option<Rgb8> {
        self.regions.colour(region)
    }

    /// The rules the level holds the copy to.
    #[must_use]
    pub fn rules(&self) -> Vec<Rule> {
        let mut rules = vec![
            Rule::Darker,
            Rule::LightColour,
            Rule::ShadowColour,
            Rule::Value,
        ];
        if matches!(self.variant.palette, Palette::Local | Palette::Bounce) {
            rules.push(Rule::LocalKept);
        }
        if self.bounces() {
            rules.push(Rule::Bounce);
        }
        rules
    }
}

/// The regions a level reads: the families on the form and on the ground, where `bounce` the
/// reflected light too.
fn asked(bounce: bool) -> Vec<Region> {
    (Region::ALL.into_iter())
        .filter(|region| bounce || *region != Region::Reflected)
        .collect()
}
