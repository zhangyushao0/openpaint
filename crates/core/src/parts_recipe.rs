//! A brush recipe as plain data for the store (`parts.rs` holds the rest of the artwork's).

use crate::brush::{BrushRecipe, Curve, Dynamics, Grain};
use crate::deposit::{Accumulation, Deposit, Paper, PaperTexture, Tip};
use crate::error::CoreError;
use crate::shape::{Rotation, Shape, Stamp};
use crate::taper::Taper;
use crate::value::invalid;

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
    /// Since S45: the shape stage (`ShapeParts::CONTINUOUS`: the round capsule).
    pub shape: ShapeParts,
}

/// The shape stage as plain data; angles in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeParts {
    pub stamp: Stamp,
    pub aspect: f32,
    pub spacing: f32,
    pub scatter: f32,
    pub rotation: Rotation,
    pub angle: f32,
    pub rotation_jitter: f32,
    pub size_jitter: f32,
}

impl ShapeParts {
    /// The continuous round capsule, as every recipe before S45.
    pub const CONTINUOUS: Self = Self {
        stamp: Stamp::Disc,
        aspect: 1.0,
        spacing: 0.0,
        scatter: 0.0,
        rotation: Rotation::Fixed,
        angle: 0.0,
        rotation_jitter: 0.0,
        size_jitter: 0.0,
    };

    fn of(shape: &Shape) -> Self {
        Self {
            stamp: shape.stamp(),
            aspect: shape.aspect(),
            spacing: shape.spacing(),
            scatter: shape.scatter(),
            rotation: shape.rotation(),
            angle: shape.angle(),
            rotation_jitter: shape.rotation_jitter(),
            size_jitter: shape.size_jitter(),
        }
    }

    /// The shape these parts describe: continuous for a spacing of 0 (then every other value
    /// must be the continuous one's).
    fn shape(&self) -> Result<Shape, CoreError> {
        if self.spacing == 0.0 {
            return if *self == Self::CONTINUOUS {
                Ok(Shape::CONTINUOUS)
            } else {
                Err(invalid("brush shape"))
            };
        }
        let rotation = (self.rotation, self.angle, self.rotation_jitter);
        let stamp = (self.stamp, self.aspect);
        Shape::stamped(
            stamp,
            self.spacing,
            self.scatter,
            rotation,
            self.size_jitter,
        )
    }
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
        let recipe = match &parts.deposit {
            None => recipe,
            Some(deposit) => {
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
                recipe.with_deposit(Deposit::Accumulate(accumulation))?
            }
        };
        recipe.with_shape(parts.shape.shape()?)
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
            shape: ShapeParts::of(self.shape()),
        }
    }
}
