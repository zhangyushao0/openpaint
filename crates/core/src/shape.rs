//! `Shape`: what a brush's tip looks like and how it is laid along the stroke (S45, DECISIONS
//! D72). With a spacing of 0 the stroke is the continuous round capsule every stroke was before
//! (D07, archived). With a spacing, the stroke is a row of stamps at equal distances along its
//! path, `n * spacing * size` canvas pixels from its start for the n-th stamp: each the tip's
//! shape (a disc,
//! a square or an ellipse of an aspect, or one of the shape textures) in its own frame, turned
//! by the rotation and jittered by a hash of the stamp's index and the stroke's seed (`jitter`),
//! so a stroke is the same on every frame, scale and device. The renderer evaluates the stamps
//! per pixel from the path; there is no stamp engine and nothing is kept between frames.

use crate::error::CoreError;
use crate::value::{invalid, is_unit};

/// Smallest spacing between stamps, as a share of the stroke's size (its largest diameter).
pub const MIN_SPACING: f32 = 0.05;
/// Largest spacing: stamps four diameters apart.
pub const MAX_SPACING: f32 = 4.0;
/// Narrowest shape: its length along its frame's x axis as a share of its width.
pub const MIN_ASPECT: f32 = 0.05;
/// Farthest a stamp may be scattered from the path, across and along, in radii.
pub const MAX_SCATTER: f32 = 4.0;
/// Most stamps of one span the evaluator takes at a pixel. A recipe whose stamps could exceed it
/// is refused (`Shape::fits`): at the spacing floor a disc (21 stamps) leaves room for a size
/// gain of about 2.3, or a scatter of about 1.4 radii.
pub const MAX_STAMPS: u32 = 48;

/// The built-in shape textures (`crates/render/assets/shape-*.bin`). A texture never changes
/// once released; another look is another texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeTexture {
    /// A chalk disc with an eroded, grainy rim.
    Chalk,
    /// A disc with holes all over.
    Grainy,
    /// A splatter of blobs.
    Splatter,
    /// A square with rough edges.
    RoughSquare,
}

impl ShapeTexture {
    pub const ALL: [Self; 4] = [Self::Chalk, Self::Grainy, Self::Splatter, Self::RoughSquare];
}

/// The shape of one stamp in its frame: across the frame's y axis it spans the stamp's
/// diameter; along its x axis `aspect` times that (a disc: 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stamp {
    Disc,
    /// A rectangle: hard corners.
    Square,
    Ellipse,
    /// A texture's alpha over the square the stamp spans.
    Texture(ShapeTexture),
}

/// Which way a stamp's frame points before its angle and jitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// The canvas's x axis.
    Fixed,
    /// The stroke's direction where the stamp lies.
    Direction,
    /// The pen's azimuth, changing evenly between samples.
    Azimuth,
}

/// The shape stage of a recipe. Angles are in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    stamp: Stamp,
    aspect: f32,
    spacing: f32,
    scatter: f32,
    rotation: Rotation,
    angle: f32,
    rotation_jitter: f32,
    size_jitter: f32,
}

impl Shape {
    /// The continuous round capsule: no stamps.
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

    /// Stamps of `stamp` (a disc's `aspect` is 1, others' in `MIN_ASPECT..=1`) every
    /// `spacing` diameters (`MIN_SPACING..=MAX_SPACING`), scattered up to `scatter` radii
    /// across and along the path (`0..=MAX_SCATTER`), turned by `rotation` plus `angle`
    /// (`-180..=180` degrees) and up to `rotation_jitter` degrees either way (`0..=180`), sized
    /// `1 ± size_jitter` (`0..=1`).
    ///
    /// # Errors
    /// `InvalidValue` when a value is outside its range.
    pub fn stamped(
        (stamp, aspect): (Stamp, f32),
        spacing: f32,
        scatter: f32,
        (rotation, angle, rotation_jitter): (Rotation, f32, f32),
        size_jitter: f32,
    ) -> Result<Self, CoreError> {
        let aspect_ok = match stamp {
            Stamp::Disc => aspect == 1.0,
            _ => (MIN_ASPECT..=1.0).contains(&aspect),
        };
        let ok = aspect_ok
            && (MIN_SPACING..=MAX_SPACING).contains(&spacing)
            && (0.0..=MAX_SCATTER).contains(&scatter)
            && (-180.0..=180.0).contains(&angle)
            && (0.0..=180.0).contains(&rotation_jitter)
            && is_unit(size_jitter);
        if !ok {
            return Err(invalid("brush shape"));
        }
        Ok(Self {
            stamp,
            aspect,
            spacing,
            scatter,
            rotation,
            angle,
            rotation_jitter,
            size_jitter,
        })
    }

    #[must_use]
    pub const fn stamp(&self) -> Stamp {
        self.stamp
    }

    #[must_use]
    pub const fn aspect(&self) -> f32 {
        self.aspect
    }

    /// The distance between stamps as a share of the stroke's size; 0: continuous.
    #[must_use]
    pub const fn spacing(&self) -> f32 {
        self.spacing
    }

    /// In radii, across and along.
    #[must_use]
    pub const fn scatter(&self) -> f32 {
        self.scatter
    }

    #[must_use]
    pub const fn rotation(&self) -> Rotation {
        self.rotation
    }

    /// Degrees.
    #[must_use]
    pub const fn angle(&self) -> f32 {
        self.angle
    }

    /// Degrees either way.
    #[must_use]
    pub const fn rotation_jitter(&self) -> f32 {
        self.rotation_jitter
    }

    #[must_use]
    pub const fn size_jitter(&self) -> f32 {
        self.size_jitter
    }

    /// True when the stroke is a row of stamps, not the continuous capsule.
    #[must_use]
    pub fn is_stamped(&self) -> bool {
        self.spacing > 0.0
    }

    /// How far a stamp reaches from its place on the path, in radii of the stroke there: its
    /// shape's farthest corner at its largest size, plus its scatter (across and along at once).
    #[must_use]
    pub fn reach(&self) -> f32 {
        let extent = match self.stamp {
            Stamp::Disc | Stamp::Ellipse => 1.0,
            Stamp::Square | Stamp::Texture(_) => self.aspect.hypot(1.0),
        };
        (1.0 + self.size_jitter) * extent + std::f32::consts::SQRT_2 * self.scatter
    }

    /// Whether every stamp that can reach a pixel fits the evaluator's `MAX_STAMPS` when the
    /// stroke's radius reaches `peak` times half its size: the stamps within the reach either
    /// way, one spacing apart, and one at each end.
    #[must_use]
    pub fn fits(&self, peak: f32) -> bool {
        !self.is_stamped() || peak * self.reach() / self.spacing + 2.0 <= MAX_STAMPS as f32
    }
}

/// Four numbers in `0..1` for stamp `index` of a stroke of `seed`: its shift along and across
/// the path, its turn and its size, each `2 u - 1` of the shape's scatter, rotation jitter and
/// size jitter. Integer hashing (PCG) and an exact conversion, so the evaluator's WGSL gives the
/// same values.
#[must_use]
pub fn jitter(seed: u32, index: u32) -> [f32; 4] {
    let mut hash = pcg(index ^ pcg(seed));
    let mut units = [0.0; 4];
    for unit in &mut units {
        *unit = (hash >> 8) as f32 * (1.0 / 16_777_216.0);
        hash = pcg(hash);
    }
    units
}

/// A well-mixed 32-bit value (PCG), as the evaluator's `pcg`.
fn pcg(value: u32) -> u32 {
    let state = value.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}
