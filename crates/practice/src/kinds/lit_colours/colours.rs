//! Colour under light's colours (S154j), by the level's palette, each drawn from the scene's
//! sequence (Oklab lightness, chroma, hue): at 入门 a white or light grey object on a grey ground, a
//! warm sun and a blue sky, or in turn a cool light and a warm sky; at 熟练 a coloured object, its
//! hue any of the wheel's twelve steps (so no colour habit forms), under a warm sun and a blue sky;
//! at 稳 a lamp of any of the twelve hues, a sky across the wheel from it, an object of any hue and a
//! ground near the lamp's hue, whose light bounces into the shadow. The light and the sky lie at
//! least 60 degrees apart and each a few steps from grey.

use crate::kinds::lit_colours::scene::Palette;
use crate::maths::colour::{Rgb8, in_gamut};
use crate::maths::random::Random;

/// The scene's colours, each as its swatch shows it: the object's own, the light's, the sky's and
/// the ground's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colours {
    pub local: Rgb8,
    pub light: Rgb8,
    pub sky: Rgb8,
    pub ground: Rgb8,
}

/// The lights' lightness and chroma, the sky's (Oklab).
const LIGHT: ([f64; 2], [f64; 2]) = ([0.86, 0.9], [0.11, 0.15]);
const WARM_LIGHT: ([f64; 2], [f64; 2]) = ([0.86, 0.9], [0.13, 0.17]);
const SKY: ([f64; 2], [f64; 2]) = ([0.76, 0.82], [0.08, 0.11]);
const SOFT_SKY: ([f64; 2], [f64; 2]) = ([0.76, 0.82], [0.06, 0.09]);
/// A warm light's hue and a blue sky's (degrees).
const WARM: f64 = 70.0;
const BLUE: f64 = 245.0;
/// Steps round the wheel, 30 degrees each.
const STEPS: u64 = 12;

/// The colours of a scene of `palette`, drawn from `random`.
pub(crate) fn coloured(palette: Palette, random: &mut Random) -> Colours {
    let white = ([0.86, 0.94], [0.0, 0.0]);
    let grey = ([0.6, 0.7], [0.0, 0.0]);
    match palette {
        Palette::WarmSun | Palette::CoolLight => {
            let warm = palette == Palette::WarmSun;
            let (light, above) = if warm { (WARM, BLUE) } else { (230.0, 65.0) };
            Colours {
                light: pick(random, LIGHT, light),
                sky: pick(random, SKY, above),
                local: pick(random, white, 0.0),
                ground: pick(random, grey, 0.0),
            }
        }
        Palette::Local => {
            let hue = step(random);
            Colours {
                light: pick(random, WARM_LIGHT, WARM),
                sky: pick(random, SOFT_SKY, BLUE),
                local: pick(random, ([0.6, 0.74], [0.07, 0.12]), hue),
                ground: pick(random, grey, 0.0),
            }
        }
        Palette::Bounce => {
            let lamp = step(random);
            let above = lamp + 180.0 + random.range(-40.0, 40.0);
            let local = 360.0 * random.unit();
            let ground = lamp + random.range(-50.0, 50.0);
            Colours {
                light: pick(random, LIGHT, lamp),
                sky: pick(random, SKY, above),
                local: pick(random, ([0.6, 0.74], [0.06, 0.12]), local),
                ground: pick(random, ([0.62, 0.72], [0.08, 0.12]), ground),
            }
        }
    }
}

/// One of the wheel's twelve hues, drawn from `random` (degrees).
fn step(random: &mut Random) -> f64 {
    (random.next_u64() % STEPS) as f64 * 30.0
}

/// A colour of hue `h` (degrees) with its lightness and chroma drawn from `random` within `l` and
/// `c` (Oklab), as much of the chroma as sRGB holds there.
fn pick(random: &mut Random, (l, c): ([f64; 2], [f64; 2]), h: f64) -> Rgb8 {
    let (l, c) = (random.range(l[0], l[1]), random.range(c[0], c[1]));
    in_gamut([l, c, h.rem_euclid(360.0)])
}
