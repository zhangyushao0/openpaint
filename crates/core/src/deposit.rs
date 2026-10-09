//! How a stroke lays its paint (DECISIONS D38, D72). `Cover`: a pixel takes the largest coverage
//! any segment of the stroke gives it, so a stroke crossing itself stays one even coverage (the
//! Ink Pen). `Accumulate`: the stroke builds up optical density along its path, so where it
//! crosses itself it darkens, up to its opacity, like graphite or an airbrush with a flow below
//! 100 %. Density is laid per radius travelled, never per sample, so more input events over the
//! same path never darken it.

use crate::brush::Curve;
use crate::error::CoreError;
use crate::value::{invalid, is_unit};

/// Largest flow, in optical density per radius travelled.
pub const MAX_FLOW: f32 = 16.0;
/// Paper texels per canvas pixel: the finest and the coarsest grain a paper can have.
pub const PAPER_SCALES: [f32; 2] = [0.25, 64.0];
/// Largest depth exponent of a paper's ridges.
pub const MAX_PAPER_DEPTH: f32 = 4.0;

/// How a stroke combines the coverage of its segments.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Deposit {
    /// The largest coverage of any segment; crossings never darken.
    #[default]
    Cover,
    /// Optical density summed along the path.
    Accumulate(Accumulation),
}

/// The cross-section of an accumulating tip: how much it lays at distance `d` (in radii) from
/// the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tip {
    /// `1 - d^8`: even across the tip, falling off only at the rim (graphite).
    Flat,
    /// `(1 - d^2)^2`: a smooth bell from the path to the rim (an airbrush).
    Bell,
}

/// An accumulating deposit. Along the path, the density at a pixel grows by the tip's
/// cross-section integrated over the radii travelled, times `flow` times the opacity
/// dynamics; the stroke's coverage there is `max_density * (1 - exp(-density))`, through the
/// paper when there is one.
#[derive(Debug, Clone, PartialEq)]
pub struct Accumulation {
    tip: Tip,
    flow: f32,
    max_density: f32,
    paper: Option<Paper>,
}

impl Accumulation {
    /// `flow` in `(0, MAX_FLOW]`; `max_density` in `(0, 1]`; a paper only under a flat tip.
    ///
    /// # Errors
    /// `InvalidValue` when a value is outside its range or a bell tip has a paper.
    pub fn new(
        tip: Tip,
        flow: f32,
        max_density: f32,
        paper: Option<Paper>,
    ) -> Result<Self, CoreError> {
        let flow_ok = flow > 0.0 && flow <= MAX_FLOW;
        let density_ok = max_density > 0.0 && max_density <= 1.0;
        let paper_ok = paper.is_none() || tip == Tip::Flat;
        if flow_ok && density_ok && paper_ok {
            Ok(Self {
                tip,
                flow,
                max_density,
                paper,
            })
        } else {
            Err(invalid("brush deposit"))
        }
    }

    #[must_use]
    pub const fn tip(&self) -> Tip {
        self.tip
    }

    #[must_use]
    pub const fn flow(&self) -> f32 {
        self.flow
    }

    #[must_use]
    pub const fn max_density(&self) -> f32 {
        self.max_density
    }

    #[must_use]
    pub const fn paper(&self) -> Option<&Paper> {
        self.paper.as_ref()
    }
}

/// The built-in paper textures. A texture never changes once released; a new look is a new
/// texture, so a stroke's paper always looks as it was drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperTexture {
    /// Fine drawing paper (ambientCG Paper001, CC0), fixed to the canvas.
    Sketch,
}

/// Paper under an accumulating flat tip: a height field fixed to the canvas, each place's
/// height its rank among all (uniform in `0..=1`). The tip touches the `contact` share of the
/// paper (over pressure), the highest ridges first: a ridge takes `dust + (1 - dust) *
/// reach^depth` of the density, `reach` how far it rises above the contact level as a share
/// of the contact; the valleys only the `dust` share.
#[derive(Debug, Clone, PartialEq)]
pub struct Paper {
    texture: PaperTexture,
    contact: Curve,
    scale: f32,
    depth: f32,
    dust: f32,
}

impl Paper {
    /// `contact` a curve over pressure (`0..=1`); `scale` texels per canvas pixel within
    /// `PAPER_SCALES`; `depth` in `(0, MAX_PAPER_DEPTH]`; `dust` in `0..=1`.
    ///
    /// # Errors
    /// `InvalidValue` when a value is outside its range.
    pub fn new(
        texture: PaperTexture,
        contact: Curve,
        scale: f32,
        depth: f32,
        dust: f32,
    ) -> Result<Self, CoreError> {
        let [finest, coarsest] = PAPER_SCALES;
        let ok = contact.last_x() <= 1.0
            && (finest..=coarsest).contains(&scale)
            && depth > 0.0
            && depth <= MAX_PAPER_DEPTH
            && is_unit(dust);
        if ok {
            Ok(Self {
                texture,
                contact,
                scale,
                depth,
                dust,
            })
        } else {
            Err(invalid("brush paper"))
        }
    }

    #[must_use]
    pub const fn texture(&self) -> PaperTexture {
        self.texture
    }

    /// The share of the paper the tip touches, over pressure.
    #[must_use]
    pub const fn contact(&self) -> &Curve {
        &self.contact
    }

    /// Texels per canvas pixel.
    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    #[must_use]
    pub const fn depth(&self) -> f32 {
        self.depth
    }

    #[must_use]
    pub const fn dust(&self) -> f32 {
        self.dust
    }
}
