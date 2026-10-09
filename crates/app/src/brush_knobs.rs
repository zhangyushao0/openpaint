//! What the brush editor's controls set (S46): the numbers of a recipe with their names and the
//! ranges their controls give, and the dynamics curves by the factor they set and what they
//! follow.

use openpaint_core::{MAX_GAIN, MAX_SPEED, MAX_TILT};

use crate::brush_fit::Curve as Mapping;

/// A number the editor sets, in the units its control shows: spacing in diameters (0:
/// continuous), taper lengths in diameters, angles in degrees, scatter in radii, grain scale and
/// sizes in canvas pixels, the rest shares from 0 to 1 (flow: density per radius travelled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Knob {
    Spacing,
    TaperStart,
    TaperEnd,
    TaperTip,
    Aspect,
    Angle,
    RotationJitter,
    SizeJitter,
    Scatter,
    Softness,
    GrainStrength,
    GrainScale,
    Flow,
    MaxDensity,
    SizeMin,
    SizeMax,
}

impl Knob {
    /// What the control is called.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Spacing => "Spacing",
            Self::TaperStart => "Taper start",
            Self::TaperEnd => "Taper end",
            Self::TaperTip => "Taper tip",
            Self::Aspect => "Aspect",
            Self::Angle => "Angle",
            Self::RotationJitter => "Rotation jitter",
            Self::SizeJitter => "Size jitter",
            Self::Scatter => "Scatter",
            Self::Softness => "Softness",
            Self::GrainStrength => "Strength",
            Self::GrainScale => "Scale",
            Self::Flow => "Flow",
            Self::MaxDensity => "Max density",
            Self::SizeMin => "Minimum",
            Self::SizeMax => "Maximum",
        }
    }

    /// True for the settings of stamps, which a continuous stroke does not have.
    pub(crate) const fn stamps_only(self) -> bool {
        matches!(
            self,
            Self::Aspect | Self::Angle | Self::RotationJitter | Self::SizeJitter | Self::Scatter
        )
    }
}

/// The values a control gives, and how its position maps to them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KnobRange {
    pub min: f32,
    pub max: f32,
    mapping: Mapping,
}

impl KnobRange {
    pub(crate) const fn new(min: f32, max: f32, mapping: Mapping) -> Self {
        Self { min, max, mapping }
    }

    /// The value at `position` (`0..=1`) along the control.
    #[must_use]
    pub fn at(&self, position: f32) -> f32 {
        let t = position.clamp(0.0, 1.0);
        let span = self.max - self.min;
        match self.mapping {
            Mapping::Linear => self.min + span * t,
            Mapping::Squared => self.min + span * t * t,
            Mapping::Whole => (self.min + span * t).round(),
        }
    }

    /// Where `value` sits along the control, `0..=1`.
    #[must_use]
    pub fn position(&self, value: f32) -> f32 {
        let span = self.max - self.min;
        if span <= 0.0 {
            return 1.0;
        }
        let t = ((value - self.min) / span).clamp(0.0, 1.0);
        match self.mapping {
            Mapping::Linear | Mapping::Whole => t,
            Mapping::Squared => t.sqrt(),
        }
    }

    /// `value` held inside the range (whole for a whole number).
    pub(crate) fn clamp(&self, value: f32) -> f32 {
        let value = value.clamp(self.min, self.max);
        match self.mapping {
            Mapping::Whole => value.round(),
            Mapping::Linear | Mapping::Squared => value,
        }
    }
}

/// Which factor a dynamics curve sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dynamic {
    Size,
    Opacity,
}

/// What a dynamics curve follows: pressure (`0..=1`), speed (canvas pixels per millisecond, up to
/// `MAX_SPEED`) or tilt (degrees from upright, up to `MAX_TILT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Pressure,
    Speed,
    Tilt,
}

impl Axis {
    pub const ALL: [Self; 3] = [Self::Pressure, Self::Speed, Self::Tilt];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pressure => "Pressure",
            Self::Speed => "Speed",
            Self::Tilt => "Tilt",
        }
    }

    /// The largest value of what the curve follows.
    #[must_use]
    pub const fn most(self) -> f32 {
        match self {
            Self::Pressure => 1.0,
            Self::Speed => MAX_SPEED,
            Self::Tilt => MAX_TILT,
        }
    }
}

/// The largest factor a curve of `dynamic` over `axis` may have: a size widens with tilt up to
/// `MAX_GAIN`, every other factor is at most 1.
#[must_use]
pub const fn curve_top(dynamic: Dynamic, axis: Axis) -> f32 {
    match (dynamic, axis) {
        (Dynamic::Size, Axis::Tilt) => MAX_GAIN,
        _ => 1.0,
    }
}
