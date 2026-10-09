//! A brush recipe as plain data for the store (`parts.rs` holds the rest of the artwork's).

use crate::brush::{BrushRecipe, Curve, Dynamics, Grain};
use crate::deposit::{Accumulation, Deposit, Paper, PaperTexture, Tip};
use crate::error::CoreError;
use crate::taper::Taper;

/// A brush recipe as plain data: curves as `[x, y]` points, tilt curves over degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeParts {
    pub softness: f32,
    pub size_pressure: Vec<[f32; 2]>,
    pub size_speed: Vec<[f32; 2]>,
    pub opacity_pressure: Vec<[f32; 2]>,
    pub opacity_speed: Vec<[f32; 2]>,
    pub grain_strength: f32,
    pub grain_scale: u32,
    /// `None` for a covering brush.
    pub deposit: Option<AccumulationParts>,
    /// Since S44: size and opacity over tilt (`[[0, 1]]`: no tilt response).
    pub size_tilt: Vec<[f32; 2]>,
    pub opacity_tilt: Vec<[f32; 2]>,
    /// Since S44: the taper's start and end lengths in diameters and its tip share (all 0: none).
    pub taper: [f32; 3],
}

/// An accumulating deposit as plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct AccumulationParts {
    pub tip: Tip,
    pub flow: f32,
    pub max_density: f32,
    pub paper: Option<PaperParts>,
}

/// A paper as plain data; `contact` a curve over pressure.
#[derive(Debug, Clone, PartialEq)]
pub struct PaperParts {
    pub texture: PaperTexture,
    pub contact: Vec<[f32; 2]>,
    pub scale: f32,
    pub depth: f32,
    pub dust: f32,
}

impl BrushRecipe {
    /// Builds a recipe from plain data, checking every value.
    ///
    /// # Errors
    /// `InvalidValue` for any curve, softness, grain, deposit or taper value out of its range.
    pub fn from_parts(parts: &RecipeParts) -> Result<Self, CoreError> {
        let dynamics = |pressure: &[[f32; 2]], speed: &[[f32; 2]], tilt: &[[f32; 2]]| {
            Dynamics::new(Curve::new(pressure)?, Curve::new(speed)?)?.with_tilt(Curve::gain(tilt)?)
        };
        let [start, end, tip] = parts.taper;
        let recipe = Self::new(
            parts.softness,
            dynamics(&parts.size_pressure, &parts.size_speed, &parts.size_tilt)?,
            dynamics(
                &parts.opacity_pressure,
                &parts.opacity_speed,
                &parts.opacity_tilt,
            )?,
            Grain::new(parts.grain_strength, parts.grain_scale)?,
        )?
        .with_taper(Taper::new(start, end, tip)?);
        let Some(deposit) = &parts.deposit else {
            return Ok(recipe);
        };
        let paper = deposit
            .paper
            .as_ref()
            .map(|paper| {
                let contact = Curve::new(&paper.contact)?;
                Paper::new(paper.texture, contact, paper.scale, paper.depth, paper.dust)
            })
            .transpose()?;
        let accumulation =
            Accumulation::new(deposit.tip, deposit.flow, deposit.max_density, paper)?;
        recipe.with_deposit(Deposit::Accumulate(accumulation))
    }

    /// The recipe as plain data.
    #[must_use]
    pub fn to_parts(&self) -> RecipeParts {
        let points = |curve: &Curve| curve.points().to_vec();
        let deposit = match self.deposit() {
            Deposit::Cover => None,
            Deposit::Accumulate(accumulation) => Some(AccumulationParts {
                tip: accumulation.tip(),
                flow: accumulation.flow(),
                max_density: accumulation.max_density(),
                paper: accumulation.paper().map(|paper| PaperParts {
                    texture: paper.texture(),
                    contact: points(paper.contact()),
                    scale: paper.scale(),
                    depth: paper.depth(),
                    dust: paper.dust(),
                }),
            }),
        };
        let taper = self.taper();
        RecipeParts {
            softness: self.softness(),
            size_pressure: points(self.size().pressure()),
            size_speed: points(self.size().speed()),
            opacity_pressure: points(self.opacity().pressure()),
            opacity_speed: points(self.opacity().speed()),
            grain_strength: self.grain().strength(),
            grain_scale: self.grain().scale(),
            deposit,
            size_tilt: points(self.size().tilt()),
            opacity_tilt: points(self.opacity().tilt()),
            taper: [taper.start(), taper.end(), taper.tip()],
        }
    }
}
