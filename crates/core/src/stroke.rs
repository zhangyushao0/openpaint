//! Finished strokes, their samples and their style.

use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use crate::error::CoreError;
use crate::id::StrokeId;
use crate::value::{CanvasPoint, Color, invalid, is_unit};

/// Largest brush diameter in canvas pixels.
pub const MAX_BRUSH_SIZE: f32 = 1000.0;
/// Most samples one stroke may hold.
pub const MAX_STROKE_SAMPLES: usize = 1 << 20;

/// One pen sample of a stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeSample {
    position: CanvasPoint,
    pressure: f32,
    tilt: f32,
    azimuth: f32,
    time_ms: u32,
}

impl StrokeSample {
    /// `pressure` in `0..=1`; `tilt` in radians from the screen normal, `0..=pi/2`; `azimuth` in
    /// radians, `-pi..=pi`; `time_ms` counted from the start of the stroke.
    ///
    /// # Errors
    /// `InvalidValue` when a value is not finite or outside its range.
    pub fn new(
        position: CanvasPoint,
        pressure: f32,
        tilt: f32,
        azimuth: f32,
        time_ms: u32,
    ) -> Result<Self, CoreError> {
        if !is_unit(pressure) {
            return Err(invalid("pressure"));
        }
        if !(0.0..=FRAC_PI_2).contains(&tilt) {
            return Err(invalid("tilt"));
        }
        if !(-PI..=PI).contains(&azimuth) {
            return Err(invalid("azimuth"));
        }
        Ok(Self {
            position,
            pressure,
            tilt,
            azimuth,
            time_ms,
        })
    }

    #[must_use]
    pub const fn position(&self) -> CanvasPoint {
        self.position
    }

    #[must_use]
    pub const fn pressure(&self) -> f32 {
        self.pressure
    }

    #[must_use]
    pub const fn tilt(&self) -> f32 {
        self.tilt
    }

    #[must_use]
    pub const fn azimuth(&self) -> f32 {
        self.azimuth
    }

    #[must_use]
    pub const fn time_ms(&self) -> u32 {
        self.time_ms
    }
}

/// How a stroke is drawn: a round brush of `size` diameter, `opacity` and `color`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    size: f32,
    opacity: f32,
    color: Color,
}

impl StrokeStyle {
    /// `size` in `(0, MAX_BRUSH_SIZE]` canvas pixels; `opacity` in `(0, 1]`.
    ///
    /// # Errors
    /// `InvalidValue` when `size` or `opacity` is not finite or outside its range.
    pub fn new(size: f32, opacity: f32, color: Color) -> Result<Self, CoreError> {
        if !(size > 0.0 && size <= MAX_BRUSH_SIZE) {
            return Err(invalid("brush size"));
        }
        if !(opacity > 0.0 && opacity <= 1.0) {
            return Err(invalid("stroke opacity"));
        }
        Ok(Self {
            size,
            opacity,
            color,
        })
    }

    #[must_use]
    pub const fn size(&self) -> f32 {
        self.size
    }

    #[must_use]
    pub const fn opacity(&self) -> f32 {
        self.opacity
    }

    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }
}

/// A finished stroke inside a layer. Built only by finishing a stroke session or by
/// `Artwork::from_storage`, so its samples are never empty and never go back in time.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    id: StrokeId,
    style: StrokeStyle,
    samples: Arc<[StrokeSample]>,
}

impl Stroke {
    pub(crate) fn new(id: StrokeId, style: StrokeStyle, samples: Arc<[StrokeSample]>) -> Self {
        Self { id, style, samples }
    }

    #[must_use]
    pub const fn id(&self) -> StrokeId {
        self.id
    }

    #[must_use]
    pub const fn style(&self) -> &StrokeStyle {
        &self.style
    }

    /// Oldest first; never empty.
    #[must_use]
    pub fn samples(&self) -> &[StrokeSample] {
        &self.samples
    }

    /// True when both strokes hold one shared sample array: one stroke seen from two artwork
    /// versions. O(1). Strokes drawn on two branches of a history may share an id, never this.
    #[must_use]
    pub fn shares_samples(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.samples, &other.samples)
    }
}

/// Checks that `next` continues after `previous` without going back in time.
pub(crate) fn check_order(
    previous: Option<&StrokeSample>,
    next: &[StrokeSample],
) -> Result<(), CoreError> {
    let mut last = previous.map(StrokeSample::time_ms);
    for sample in next {
        if let Some(previous_ms) = last
            && sample.time_ms < previous_ms
        {
            return Err(CoreError::SampleOrder {
                previous_ms,
                next_ms: sample.time_ms,
            });
        }
        last = Some(sample.time_ms);
    }
    Ok(())
}
