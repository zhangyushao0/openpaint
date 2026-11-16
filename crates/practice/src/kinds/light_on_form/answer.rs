//! Light on form's answer (S153): what the light paints at each point of the frame, by the model.
//! The eye's ray meets the form or the ground (`Scene::cast`); the point is in the light family
//! where its surface faces the sun (`n · l > 0`) and the ray from it towards the sun leaves the
//! scene unhit, else in the shadow family. Inside the light family a form's tone follows its turn
//! to the light: light from `LIGHT` up, halftone up to `HALFTONE`, either between (where halftone
//! begins is the model's choice). Inside the shadow family at 稳: the core, darkest (a round form's
//! turn just past its terminator, `CORE`; a flat face's strip `CORE_STRIP` wide along the edge it
//! meets a lit face at), the reflected light, dark (facing the lit ground, `REFLECTED`), and the
//! contact, darkest (within `CONTACT` of where the form meets the ground). A band of `SOFT` either
//! side of a round form's terminator takes either family: a soft edge.
//!
//! The answer is drawn posterised in the level's greys by the lit picture (`lit_picture`), the
//! picture Compare lays.

use crate::compositions::value_masses::Value;
use crate::kinds::light_on_form::scene::{LightTask, Tones};
use crate::kinds::objects::object_body::GROUND;
use crate::kinds::objects::object_picture::lit_picture;
use crate::kinds::tools::{DARKEST, Pen};
use crate::marks::mark::LinePen;
use crate::maths::colour::{Rgb8, oklab};
use crate::maths::path::{Point, distance, nearest_on_segment};
use crate::maths::vector::{dot, times};

/// A form's turn to the light: light from here up, halftone up to here.
pub const LIGHT: f64 = 0.55;
pub const HALFTONE: f64 = 0.3;
/// Where the answer's picture parts light from halftone, between the two.
const PARTED: f64 = (LIGHT + HALFTONE) / 2.0;
/// A round form's turn either side of its terminator that takes either family.
pub const SOFT: f64 = 0.1;
/// A round form's core: its turn past the terminator down to this.
pub const CORE: f64 = -0.25;
/// A flat face's core: a strip this wide along its lit neighbour (frame units).
pub const CORE_STRIP: f64 = 0.04;
/// The reflected light: the shadow family facing down at least this much.
pub const REFLECTED: f64 = 0.3;
/// The contact: within this of where the form meets the ground (frame units).
pub const CONTACT: f64 = 0.03;

/// The light family or the shadow family: the two masses a form's values fall in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mass {
    Light,
    Shadow,
}

/// The greys light on form paints in, darkest first: the darkest (最暗, S153) and value masses'
/// three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Grey {
    Darkest,
    Dark,
    Middle,
    Light,
}

impl Grey {
    pub const ALL: [Self; 4] = [Self::Darkest, Self::Dark, Self::Middle, Self::Light];

    /// Its grey.
    #[must_use]
    pub const fn grey(self) -> Rgb8 {
        match self {
            Self::Darkest => DARKEST,
            Self::Dark => Value::Dark.grey(),
            Self::Middle => Value::Middle.grey(),
            Self::Light => Value::Light.grey(),
        }
    }

    /// The pen that paints it.
    #[must_use]
    pub const fn pen(self) -> Pen {
        match self {
            Self::Darkest => Pen::Darkest,
            Self::Dark => Pen::Dark,
            Self::Middle => Pen::Middle,
            Self::Light => Pen::Light,
        }
    }

    /// Its family.
    #[must_use]
    pub const fn family(self) -> Mass {
        match self {
            Self::Darkest | Self::Dark => Mass::Shadow,
            Self::Middle | Self::Light => Mass::Light,
        }
    }

    /// The hatching that marks where it should be.
    pub(crate) const fn hatch(self) -> LinePen {
        match self {
            Self::Darkest => LinePen::HatchDarkest,
            Self::Dark => LinePen::Hatch(Value::Dark),
            Self::Middle => LinePen::Hatch(Value::Middle),
            Self::Light => LinePen::Hatch(Value::Light),
        }
    }

    /// The tone of `tones` a copy's pixel `rgb` was painted in: the nearest by lightness; none
    /// where it lies nearer the paper's white (left bare).
    #[must_use]
    pub fn painted(rgb: Rgb8, tones: &[Self]) -> Option<Self> {
        let lightness = oklab(rgb)[0];
        let of = |tone: Self| oklab(tone.grey())[0];
        let paper = f64::midpoint(of(Self::Light), 1.0);
        if lightness >= paper {
            return None;
        }
        let off = |tone: &Self| (of(*tone) - lightness).abs();
        tones
            .iter()
            .copied()
            .min_by(|a, b| off(a).total_cmp(&off(b)))
    }
}

impl Tones {
    /// The greys a level paints with, darkest first.
    #[must_use]
    pub const fn tones(self) -> &'static [Grey] {
        match self {
            Self::Two => &[Grey::Dark, Grey::Light],
            Self::Three => &[Grey::Dark, Grey::Middle, Grey::Light],
            Self::Four => &Grey::ALL,
        }
    }
}

/// What lies inside a family where the level tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Inner {
    Plain,
    /// The light grazing a form: halftone.
    Halftone,
    /// Between light and halftone: either.
    Either,
    Core,
    Reflected,
    Contact,
}

/// What the light paints at a point: its family and tone at the task's level, whether either
/// family is taken there, whether it lies on the form, and what of the family it is; the surface
/// it lies on, its turn to the light and whether it is round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Shading {
    pub(crate) family: Mass,
    pub(crate) tone: Grey,
    pub(crate) soft: bool,
    pub(crate) on_form: bool,
    pub(crate) inner: Inner,
    pub(crate) surface: u32,
    pub(crate) turned: f64,
    pub(crate) round: bool,
}

impl Shading {
    /// Whether it is the cast shadow: the ground in the shadow family.
    pub(crate) fn cast(&self) -> bool {
        !self.on_form && self.family == Mass::Shadow
    }
}

impl LightTask {
    /// What the light paints at the frame point `p`; none above the horizon (never in the frame).
    pub(crate) fn shading(&self, p: Point) -> Option<Shading> {
        let d = self.ray(p);
        let hit = self.scene.cast(([0.0; 3], d))?;
        let at = times(d, hit.t);
        let (towards, reach) = self.light.from(at);
        let turned = dot(hit.normal, towards);
        let lit = self.scene.lit((at, hit.normal), (towards, reach));
        let on_form = hit.surface != GROUND;
        let four = self.lit.tones == Tones::Four;
        let (tone, inner) = if lit {
            match self.lit.tones {
                Tones::Two => (Grey::Light, Inner::Plain),
                _ if turned <= HALFTONE => (Grey::Middle, Inner::Halftone),
                _ if turned < PARTED => (Grey::Middle, Inner::Either),
                _ if turned < LIGHT => (Grey::Light, Inner::Either),
                _ => (Grey::Light, Inner::Plain),
            }
        } else if four && self.touches(p) {
            (Grey::Darkest, Inner::Contact)
        } else if four && on_form && self.core(p, hit.round, turned) {
            (Grey::Darkest, Inner::Core)
        } else if four && on_form && -hit.normal[1] >= REFLECTED {
            (Grey::Dark, Inner::Reflected)
        } else {
            (Grey::Dark, Inner::Plain)
        };
        Some(Shading {
            family: if lit { Mass::Light } else { Mass::Shadow },
            tone,
            soft: hit.round && turned.abs() < SOFT,
            on_form,
            inner,
            surface: hit.surface,
            turned,
            round: hit.round,
        })
    }

    /// Whether the frame point `p` lies within `CONTACT` of where the form meets the ground.
    fn touches(&self, p: Point) -> bool {
        self.contact.iter().any(|line| match line.as_slice() {
            [only] => distance(*only, p) <= CONTACT,
            _ => (line.windows(2))
                .any(|pair| distance(nearest_on_segment(pair[0], pair[1], p), p) <= CONTACT),
        })
    }

    /// Whether the shadow-family point `p` of the form, turned `turned` to the light, is its core:
    /// a round form's turn just past its terminator; a flat face's strip along a lit neighbour.
    fn core(&self, p: Point, round: bool, turned: f64) -> bool {
        if round {
            return turned > CORE;
        }
        (self.creases.iter()).any(|[a, b]| distance(nearest_on_segment(*a, *b, p), p) <= CORE_STRIP)
    }

    /// The answer posterised in the level's greys, `side` pixels square (RGBA, opaque).
    #[must_use]
    pub fn picture(&self, side: u32) -> Vec<u8> {
        let surface = |p: Point| {
            let shading = self.shading(p)?;
            let grey = f64::from(shading.tone.grey()[0]) / 255.0;
            Some((shading.tone as u32, grey))
        };
        lit_picture((side, [[0.0; 2], [1.0; 2]], [1.0; 3]), surface)
    }

    /// What the light paints at each pixel of a frame `size` pixels (row by row, each at its
    /// middle; the frame one to its longer side).
    pub(crate) fn map(&self, size: [u32; 2]) -> Vec<Option<Shading>> {
        let unit = f64::from(size[0].max(size[1]).max(1));
        let [w, h] = size.map(|side| side as usize);
        (0..w * h)
            .map(|at| {
                let middle = |v: usize| (v as f64 + 0.5) / unit;
                self.shading([middle(at % w), middle(at / w)])
            })
            .collect()
    }
}
