//! `StrokeSession`: a stroke in progress. It lives beside the artwork, never inside it, until
//! `finish` commits it as one new version.

use std::sync::Arc;

use crate::artwork::Artwork;
use crate::command::{Applied, Change};
use crate::error::CoreError;
use crate::id::LayerId;
use crate::region::{RegionMask, check_mask};
use crate::stroke::{
    MAX_STROKE_SAMPLES, Stroke, StrokeBlend, StrokeSample, StrokeStyle, StrokeTool, blend,
    check_order,
};
use crate::value::invalid;

/// The in-progress stroke: what the renderer draws on top of the base artwork.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveStroke {
    layer: LayerId,
    /// The layer's position in the base artwork, which cannot change under the session.
    index: usize,
    style: StrokeStyle,
    /// The layer's Alpha Lock when the stroke began.
    alpha_locked: bool,
    samples: Vec<StrokeSample>,
    /// The selection the stroke is drawn inside.
    clip: Option<RegionMask>,
}

impl ActiveStroke {
    /// The selection the stroke is drawn inside, if any.
    #[must_use]
    pub const fn clip(&self) -> Option<&RegionMask> {
        self.clip.as_ref()
    }

    /// The layer the stroke will land on.
    #[must_use]
    pub const fn layer(&self) -> LayerId {
        self.layer
    }

    #[must_use]
    pub const fn style(&self) -> &StrokeStyle {
        &self.style
    }

    /// How the stroke combines with its layer.
    #[must_use]
    pub const fn blend(&self) -> StrokeBlend {
        blend(self.style.tool(), self.alpha_locked)
    }

    /// Oldest first; never empty.
    #[must_use]
    pub fn samples(&self) -> &[StrokeSample] {
        &self.samples
    }
}

/// A stroke being drawn on the active layer of `base`.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeSession {
    base: Artwork,
    active: ActiveStroke,
}

impl Artwork {
    /// Starts a stroke on the active layer with its first sample. A paint stroke on a layer
    /// with Alpha Lock on paints only over the layer's paint, for its whole life.
    ///
    /// # Errors
    /// `LayerLocked` when the active layer is locked; `AlphaLocked` for the eraser on a layer
    /// whose alpha is locked (it could take nothing away).
    pub fn begin_stroke(
        &self,
        style: StrokeStyle,
        first: StrokeSample,
    ) -> Result<StrokeSession, CoreError> {
        let layer = self.active();
        let index = self.editable_layer(layer)?;
        let alpha_locked = self.state().layers[index].props.alpha_locked;
        if alpha_locked && style.tool() == StrokeTool::Erase {
            return Err(CoreError::AlphaLocked(layer));
        }
        Ok(StrokeSession {
            base: self.clone(),
            active: ActiveStroke {
                layer,
                index,
                style,
                alpha_locked,
                samples: vec![first],
                clip: None,
            },
        })
    }
}

impl StrokeSession {
    /// This stroke drawn inside the selection `clip`: it changes only the pixels of `clip`,
    /// while it is drawn and once committed.
    ///
    /// # Errors
    /// `InvalidValue` when the mask leaves the canvas.
    pub fn clipped(mut self, clip: RegionMask) -> Result<Self, CoreError> {
        check_mask(self.base.canvas(), &clip)?;
        self.active.clip = Some(clip);
        Ok(self)
    }

    /// The artwork the stroke started on.
    #[must_use]
    pub const fn base(&self) -> &Artwork {
        &self.base
    }

    #[must_use]
    pub const fn active(&self) -> &ActiveStroke {
        &self.active
    }

    /// Appends samples, oldest first. All or nothing: on error the stroke is unchanged.
    ///
    /// # Errors
    /// `SampleOrder` when a sample is earlier than the one before it; `SampleLimit` when the
    /// stroke would exceed `MAX_STROKE_SAMPLES`.
    pub fn extend(&mut self, samples: &[StrokeSample]) -> Result<(), CoreError> {
        if self.active.samples.len() + samples.len() > MAX_STROKE_SAMPLES {
            return Err(CoreError::SampleLimit {
                max: MAX_STROKE_SAMPLES,
            });
        }
        check_order(self.active.samples.last(), samples)?;
        self.active.samples.extend_from_slice(samples);
        Ok(())
    }

    /// Replaces the samples from index `from` on with `samples`, oldest first: a stroke whose
    /// end is redrawn as it lifts (stabilisation catching up with the pen). The first sample
    /// stays. All or nothing.
    ///
    /// # Errors
    /// `InvalidValue` when `from` is 0 or beyond the samples; `SampleOrder` and `SampleLimit`
    /// as for `extend`.
    pub fn revise(&mut self, from: usize, samples: &[StrokeSample]) -> Result<(), CoreError> {
        let held = &self.active.samples;
        if from == 0 || from > held.len() {
            return Err(invalid("revised samples"));
        }
        if from + samples.len() > MAX_STROKE_SAMPLES {
            return Err(CoreError::SampleLimit {
                max: MAX_STROKE_SAMPLES,
            });
        }
        check_order(held.get(from - 1), samples)?;
        self.active.samples.truncate(from);
        self.active.samples.extend_from_slice(samples);
        Ok(())
    }

    /// Commits the stroke to its layer as one new version of the base artwork.
    #[must_use]
    pub fn finish(self) -> Applied {
        let mut next = self.base.state().clone();
        let id = next.ids.take_stroke();
        let active = self.active;
        let samples = Arc::from(active.samples);
        let stroke = Stroke::new(id, active.style, active.alpha_locked, samples, active.clip);
        let layer = &mut next.layers[active.index];
        layer.strokes = layer.strokes.pushed(stroke);
        Applied {
            artwork: Artwork::from_state(next),
            change: Change::Content,
        }
    }

    /// Drops the stroke; returns the base artwork, which never saw it.
    #[must_use]
    pub fn cancel(self) -> Artwork {
        self.base
    }
}
