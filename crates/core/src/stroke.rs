//! Finished strokes, their samples and their style.

use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use crate::brush::BrushRecipe;
use crate::error::CoreError;
use crate::id::StrokeId;
use crate::region::RegionMask;
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

/// What a stroke does to its layer: lay paint on it, take paint away, or carry the paint
/// already there along its path. All use any recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeTool {
    #[default]
    Paint,
    /// Removes coverage times opacity from the layer (destination-out); the colour is unused.
    Erase,
    /// Picks up the layer's own colour (transparency too) under the tip and drags it along the
    /// path, by the opacity as strength; the colour is unused. What it does depends on the
    /// layer's pixels under it when it was drawn.
    Smudge,
}

/// How a stroke combines with the layer under it, fixed when the stroke begins: the tool, and
/// for paint and smudge whether the layer's Alpha Lock was on then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeBlend {
    /// Paint over the layer (source over).
    Over,
    /// Paint only where the layer already has paint, keeping its alpha (source atop).
    Atop,
    /// Take paint away (destination out).
    Erase,
    /// Carry the layer's colour and transparency along the path.
    Smudge,
    /// Carry the layer's colour along the path, keeping its alpha (Alpha Lock).
    SmudgeLocked,
}

/// How a stroke is drawn: its tool, its brush recipe (a snapshot, shared between strokes drawn
/// with the same brush), its largest diameter `size`, its `opacity` and `color`, and the `seed`
/// its stamps' jitter follows (S45: `shape::jitter`; a mirrored copy and a reshaped stroke keep
/// it). Cloning is cheap.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeStyle {
    tool: StrokeTool,
    recipe: Arc<BrushRecipe>,
    size: f32,
    opacity: f32,
    color: Color,
    seed: u32,
}

impl StrokeStyle {
    /// `size` in `(0, MAX_BRUSH_SIZE]` canvas pixels; `opacity` in `(0, 1]`.
    ///
    /// # Errors
    /// `InvalidValue` when `size` or `opacity` is not finite or outside its range.
    pub fn new(
        recipe: Arc<BrushRecipe>,
        size: f32,
        opacity: f32,
        color: Color,
    ) -> Result<Self, CoreError> {
        if !(size > 0.0 && size <= MAX_BRUSH_SIZE) {
            return Err(invalid("brush size"));
        }
        if !(opacity > 0.0 && opacity <= 1.0) {
            return Err(invalid("stroke opacity"));
        }
        Ok(Self {
            tool: StrokeTool::Paint,
            recipe,
            size,
            opacity,
            color,
            seed: 0,
        })
    }

    /// This style with `tool`; `new` paints.
    #[must_use]
    pub fn with_tool(self, tool: StrokeTool) -> Self {
        Self { tool, ..self }
    }

    /// This style jittering its stamps by `seed`; `new` takes 0.
    #[must_use]
    pub fn with_seed(self, seed: u32) -> Self {
        Self { seed, ..self }
    }

    /// This style painting `color`.
    pub(crate) fn with_color(self, color: Color) -> Self {
        Self { color, ..self }
    }

    #[must_use]
    pub const fn tool(&self) -> StrokeTool {
        self.tool
    }

    #[must_use]
    pub fn recipe(&self) -> &Arc<BrushRecipe> {
        &self.recipe
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

    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }
}

/// A finished stroke inside a layer. Built only by finishing a stroke session or by
/// `Artwork::from_storage`, so its samples are never empty and never go back in time.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    id: StrokeId,
    style: StrokeStyle,
    /// The layer's Alpha Lock when the stroke began; never with the eraser.
    alpha_locked: bool,
    samples: Arc<[StrokeSample]>,
    /// The selection the stroke was drawn inside: it changes only these pixels, for good.
    clip: Option<RegionMask>,
}

impl Stroke {
    pub(crate) fn new(
        id: StrokeId,
        style: StrokeStyle,
        alpha_locked: bool,
        samples: Arc<[StrokeSample]>,
        clip: Option<RegionMask>,
    ) -> Self {
        Self {
            id,
            style,
            alpha_locked,
            samples,
            clip,
        }
    }

    /// The same stroke under another id (a duplicated layer's copy); samples and clip shared.
    pub(crate) fn renumbered(&self, id: StrokeId) -> Self {
        Self { id, ..self.clone() }
    }

    /// The selection the stroke was drawn inside, if any: it paints (or erases) only there.
    #[must_use]
    pub const fn clip(&self) -> Option<&RegionMask> {
        self.clip.as_ref()
    }

    #[must_use]
    pub const fn id(&self) -> StrokeId {
        self.id
    }

    #[must_use]
    pub const fn style(&self) -> &StrokeStyle {
        &self.style
    }

    /// How the stroke combines with its layer.
    #[must_use]
    pub const fn blend(&self) -> StrokeBlend {
        blend(self.style.tool, self.alpha_locked)
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

/// The blend of a stroke of `tool` begun on a layer whose Alpha Lock was `alpha_locked`.
pub(crate) const fn blend(tool: StrokeTool, alpha_locked: bool) -> StrokeBlend {
    match (tool, alpha_locked) {
        (StrokeTool::Erase, _) => StrokeBlend::Erase,
        (StrokeTool::Paint, true) => StrokeBlend::Atop,
        (StrokeTool::Paint, false) => StrokeBlend::Over,
        (StrokeTool::Smudge, true) => StrokeBlend::SmudgeLocked,
        (StrokeTool::Smudge, false) => StrokeBlend::Smudge,
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
