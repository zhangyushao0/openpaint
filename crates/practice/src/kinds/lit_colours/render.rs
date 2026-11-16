//! Colour under light's model (S154j), in linear light, at each point of the frame: the light
//! family takes the light's colour by its turn to it and the sky's everywhere, `local × (light ×
//! POWER × n·l + sky × AMBIENT)`; the shadow family the sky's alone, `local × sky × AMBIENT`, and
//! at 稳 the light bounced off the lit ground besides, `ground × light × POWER × l_y × BOUNCE ×
//! n·down`; the ground alike in its own colour (Gurney's local colour modified by the light and
//! the surroundings). Its render is one answer, never the only one: a model is not the light. The
//! render is drawn by the lit picture (`coloured_picture`), antialiased where its surfaces or its
//! regions part.

use crate::kinds::light_on_form::answer::Mass;
use crate::kinds::lit_colours::regions::Region;
use crate::kinds::lit_colours::scene::{ColourTask, Palette};
use crate::kinds::objects::object_picture::coloured_picture;
use crate::maths::colour::{Rgb8, from_oklab, linear, oklab_of_linear};
use crate::maths::path::Point;

/// The light's strength on a face square to it, the sky's everywhere, and the share of the light
/// the lit ground bounces up into the shadow (稳).
const POWER: f64 = 1.3;
const AMBIENT: f64 = 0.25;
const BOUNCE: f64 = 1.5;
/// The render each region's model colour is read from, pixels square.
const MODEL_SIDE: u32 = 256;
/// The most cells a region's colour is first tried on.
const SAMPLED: usize = 400;

/// `rgb` in linear light, 0 to 1 a channel.
pub(crate) fn linear_of(rgb: Rgb8) -> [f64; 3] {
    rgb.map(|c| linear(f64::from(c) / 255.0))
}

/// The 8-bit colour of linear light `c`, each channel clipped to sRGB's range.
pub(crate) fn rgb_of(c: [f64; 3]) -> Rgb8 {
    from_oklab(oklab_of_linear(c.map(|v| v.max(0.0))))
}

impl ColourTask {
    /// The model's colour at the frame point `p` (linear light), its surface and its region; none
    /// above the horizon (never in the frame).
    pub(crate) fn lit_at(&self, p: Point) -> Option<(u32, [f64; 3], Region)> {
        let shading = self.task.shading(p)?;
        let bounce = self.variant.palette == Palette::Bounce;
        let region = Region::of(&shading, bounce);
        let colours = &self.colours;
        let own = linear_of(if shading.on_form {
            colours.local
        } else {
            colours.ground
        });
        let [light, sky] = [colours.light, colours.sky].map(linear_of);
        let (towards, _) = self.task.light.from([0.0; 3]);
        let mut falls: [f64; 3] = sky.map(|s| s * AMBIENT);
        if shading.family == Mass::Light {
            let turned = shading.turned.max(0.0);
            falls = std::array::from_fn(|k| falls[k] + light[k] * POWER * turned);
        } else if bounce && shading.on_form {
            let down = self.facing_down(p);
            let ground = linear_of(colours.ground);
            let up = towards[1].max(0.0) * POWER * BOUNCE * down;
            falls = std::array::from_fn(|k| falls[k] + ground[k] * light[k] * up);
        }
        let colour = std::array::from_fn(|k| own[k] * falls[k]);
        Some((shading.surface, colour, region))
    }

    /// How far the form's surface at the frame point `p` faces down (0 to 1).
    fn facing_down(&self, p: Point) -> f64 {
        let hit = self.task.scene.cast(([0.0; 3], self.task.ray(p)));
        hit.map_or(0.0, |hit| (-hit.normal[1]).max(0.0))
    }

    /// The model's render, `side` pixels square (RGBA, opaque): one answer.
    #[must_use]
    pub fn render(&self, side: u32) -> Vec<u8> {
        let surface = |p: Point| {
            let (surface, colour, region) = self.lit_at(p)?;
            let shown = rgb_of(colour).map(|c| f64::from(c) / 255.0);
            let place = Region::ALL.iter().position(|r| *r == region).unwrap_or(0);
            Some((surface * 8 + place as u32, shown))
        };
        coloured_picture((side, [[0.0; 2], [1.0; 2]], [1.0; 3]), surface)
    }

    /// The model's colour of each region held: its render (`MODEL_SIDE` square) read as a copy is.
    pub(crate) fn model_colours(&self) -> Vec<Rgb8> {
        let render = self.render(MODEL_SIDE);
        (self
            .regions
            .read([MODEL_SIDE; 2], &render, None)
            .into_iter())
        .map(|colour| colour.unwrap_or([128; 3]))
        .collect()
    }

    /// The model's colour of each region held as a few of its inner cells give it (the median of
    /// at most `SAMPLED` of them, evenly through it): what a colouring is first tried on.
    pub(crate) fn sampled_colours(&self) -> Vec<Rgb8> {
        (self.regions.held.iter())
            .map(|region| {
                let middles = self.regions.inner_middles(*region);
                let every = middles.len().div_ceil(SAMPLED).max(1);
                let colours: Vec<Rgb8> = (middles.into_iter().step_by(every))
                    .filter_map(|p| self.lit_at(p).map(|(_, c, _)| rgb_of(c)))
                    .collect();
                crate::colour_areas::areas_read::median(colours).unwrap_or([128; 3])
            })
            .collect()
    }

    /// A copy painted flat, each region in its colour of `colours` (the model's where it names
    /// none), `side` pixels square (RGBA, opaque): what the tests and the contact sheet paint.
    #[must_use]
    pub fn flat(&self, colours: &[(Region, Rgb8)], side: u32) -> Vec<u8> {
        let side = side.max(1) as usize;
        let mut pixels = Vec::with_capacity(side * side * 4);
        for at in 0..side * side {
            let middle = |v: usize| (v as f64 + 0.5) / side as f64;
            let p = [middle(at % side), middle(at / side)];
            let region =
                (self.task.shading(p)).map_or(Region::Ground, |s| Region::of(&s, self.bounces()));
            let given = colours.iter().find(|(r, _)| *r == region).map(|(_, c)| *c);
            let [r, g, b] = given.or_else(|| self.model(region)).unwrap_or([128; 3]);
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
        pixels
    }

    /// Whether its shadow family tells the reflected light apart (稳).
    pub(crate) fn bounces(&self) -> bool {
        self.variant.palette == Palette::Bounce
    }
}
