//! `BrushRecipe`: how a brush turns pen samples into paint (DECISIONS D07). A recipe has fixed
//! stages with typed values: the edge of its round shape, size and opacity dynamics over pressure
//! and speed, grain, and how its segments deposit paint (`deposit.rs`). The renderer evaluates it
//! continuously per pixel; there are no dabs. Every stroke keeps the recipe it was drawn with, so
//! a brush that changes later never changes strokes already drawn.

use crate::deposit::Deposit;
use crate::error::CoreError;
use crate::value::{invalid, is_unit};

/// Most points one curve may hold.
pub const MAX_CURVE_POINTS: usize = 8;
/// Fastest pen speed a speed curve can name, in canvas pixels per millisecond.
pub const MAX_SPEED: f32 = 50.0;
/// Largest grain cell, in canvas pixels.
pub const MAX_GRAIN_SCALE: u32 = 64;

/// A response curve: points `[x, y]` joined by straight lines, flat before the first point and
/// after the last. `x` grows strictly from point to point; `y` is a factor in `0..=1`.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    points: Vec<[f32; 2]>,
}

impl Curve {
    /// # Errors
    /// `InvalidValue` unless there are 1 to `MAX_CURVE_POINTS` points, every `x` is finite, at
    /// least 0 and above the one before, and every `y` lies in `0..=1`.
    pub fn new(points: &[[f32; 2]]) -> Result<Self, CoreError> {
        let count = (1..=MAX_CURVE_POINTS).contains(&points.len());
        let xs = points.iter().all(|[x, _]| x.is_finite() && *x >= 0.0)
            && points.windows(2).all(|pair| pair[0][0] < pair[1][0]);
        let ys = points.iter().all(|[_, y]| is_unit(*y));
        if count && xs && ys {
            Ok(Self {
                points: points.to_vec(),
            })
        } else {
            Err(invalid("brush curve"))
        }
    }

    /// The same factor everywhere.
    fn flat(y: f32) -> Self {
        Self {
            points: vec![[0.0, y]],
        }
    }

    #[must_use]
    pub fn points(&self) -> &[[f32; 2]] {
        &self.points
    }

    /// The factor at `x`. Between two points it is `y0 + (y1 - y0) * t`, `t` the share of the
    /// way from `x0` to `x1`: a curve through `[0, 0]` and `[1, 1]` gives back `x` exactly.
    #[must_use]
    pub fn at(&self, x: f32) -> f32 {
        let [first, rest @ ..] = self.points.as_slice() else {
            return 1.0;
        };
        if x <= first[0] {
            return first[1];
        }
        let mut from = *first;
        for to in rest {
            if x <= to[0] {
                let t = (x - from[0]) / (to[0] - from[0]);
                return from[1] + (to[1] - from[1]) * t;
            }
            from = *to;
        }
        from[1]
    }

    /// True when the factor is the same everywhere.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.points.windows(2).all(|pair| pair[0][1] == pair[1][1])
    }

    pub(crate) fn last_x(&self) -> f32 {
        self.points.last().map_or(0.0, |[x, _]| *x)
    }
}

/// A factor that follows pressure and speed: the product of one curve over pressure (`0..=1`)
/// and one over speed (canvas pixels per millisecond, up to `MAX_SPEED`).
#[derive(Debug, Clone, PartialEq)]
pub struct Dynamics {
    pressure: Curve,
    speed: Curve,
}

impl Dynamics {
    /// # Errors
    /// `InvalidValue` when the pressure curve reaches beyond 1 or the speed curve beyond
    /// `MAX_SPEED`.
    pub fn new(pressure: Curve, speed: Curve) -> Result<Self, CoreError> {
        if pressure.last_x() <= 1.0 && speed.last_x() <= MAX_SPEED {
            Ok(Self { pressure, speed })
        } else {
            Err(invalid("brush dynamics"))
        }
    }

    #[must_use]
    pub const fn pressure(&self) -> &Curve {
        &self.pressure
    }

    #[must_use]
    pub const fn speed(&self) -> &Curve {
        &self.speed
    }

    /// The factor at `pressure` and `speed`.
    #[must_use]
    pub fn at(&self, pressure: f32, speed: f32) -> f32 {
        self.pressure.at(pressure) * self.speed.at(speed)
    }
}

/// Paper grain fixed to the canvas: where it is strong, less paint lands. `strength` in `0..=1`
/// (0: none); `scale` is the side of its coarse cells in canvas pixels, `1..=MAX_GRAIN_SCALE`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grain {
    strength: f32,
    scale: u32,
}

impl Grain {
    pub const NONE: Self = Self {
        strength: 0.0,
        scale: 1,
    };

    /// # Errors
    /// `InvalidValue` when `strength` is outside `0..=1` or `scale` outside its range.
    pub fn new(strength: f32, scale: u32) -> Result<Self, CoreError> {
        if is_unit(strength) && (1..=MAX_GRAIN_SCALE).contains(&scale) {
            Ok(Self { strength, scale })
        } else {
            Err(invalid("brush grain"))
        }
    }

    #[must_use]
    pub const fn strength(self) -> f32 {
        self.strength
    }

    #[must_use]
    pub const fn scale(self) -> u32 {
        self.scale
    }
}

/// A brush as data. The diameter at a sample is the stroke's size times the size dynamics. A
/// covering brush's strength there is the stroke's opacity times the opacity dynamics times the
/// grain; an accumulating one lays the opacity dynamics times its flow as density and is capped
/// by the stroke's opacity. `softness` in `0..=1` shapes a covering brush's edge: 0 is a hard
/// edge (one pixel of antialiasing), 1 fades from the centre line to the edge.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushRecipe {
    softness: f32,
    size: Dynamics,
    opacity: Dynamics,
    grain: Grain,
    deposit: Deposit,
}

impl BrushRecipe {
    /// A covering brush (`Deposit::Cover`); `with_deposit` makes it accumulate.
    ///
    /// # Errors
    /// `InvalidValue` when `softness` is outside `0..=1`.
    pub fn new(
        softness: f32,
        size: Dynamics,
        opacity: Dynamics,
        grain: Grain,
    ) -> Result<Self, CoreError> {
        if is_unit(softness) {
            Ok(Self {
                softness,
                size,
                opacity,
                grain,
                deposit: Deposit::Cover,
            })
        } else {
            Err(invalid("brush softness"))
        }
    }

    /// This recipe depositing by `deposit`. An accumulating tip has its own cross-section, so
    /// it takes no softness.
    ///
    /// # Errors
    /// `InvalidValue` for an accumulating deposit on a recipe with softness.
    pub fn with_deposit(self, deposit: Deposit) -> Result<Self, CoreError> {
        if matches!(deposit, Deposit::Accumulate(_)) && self.softness != 0.0 {
            return Err(invalid("brush deposit"));
        }
        Ok(Self { deposit, ..self })
    }

    /// The plain round brush: a hard edge, the diameter in proportion to pressure, full
    /// strength everywhere, no grain.
    #[must_use]
    pub fn plain() -> Self {
        Self {
            softness: 0.0,
            size: Dynamics {
                pressure: Curve {
                    points: vec![[0.0, 0.0], [1.0, 1.0]],
                },
                speed: Curve::flat(1.0),
            },
            opacity: Dynamics {
                pressure: Curve::flat(1.0),
                speed: Curve::flat(1.0),
            },
            grain: Grain::NONE,
            deposit: Deposit::Cover,
        }
    }

    #[must_use]
    pub const fn softness(&self) -> f32 {
        self.softness
    }

    #[must_use]
    pub const fn size(&self) -> &Dynamics {
        &self.size
    }

    #[must_use]
    pub const fn opacity(&self) -> &Dynamics {
        &self.opacity
    }

    #[must_use]
    pub const fn grain(&self) -> Grain {
        self.grain
    }

    #[must_use]
    pub const fn deposit(&self) -> &Deposit {
        &self.deposit
    }

    /// True when speed changes anything, so evaluating a stroke needs its pen speed.
    #[must_use]
    pub fn uses_speed(&self) -> bool {
        !self.size.speed.is_flat() || !self.opacity.speed.is_flat()
    }
}
