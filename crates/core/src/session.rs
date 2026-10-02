//! `StrokeSession`: a stroke in progress. It lives beside the artwork, never inside it, until
//! `finish` commits it as one new version.

use std::sync::Arc;

use crate::artwork::Artwork;
use crate::command::{Applied, Change};
use crate::error::CoreError;
use crate::id::LayerId;
use crate::stroke::{MAX_STROKE_SAMPLES, Stroke, StrokeSample, StrokeStyle, check_order};

/// The in-progress stroke: what the renderer draws on top of the base artwork.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveStroke {
    layer: LayerId,
    /// The layer's position in the base artwork, which cannot change under the session.
    index: usize,
    style: StrokeStyle,
    samples: Vec<StrokeSample>,
}

impl ActiveStroke {
    /// The layer the stroke will land on.
    #[must_use]
    pub const fn layer(&self) -> LayerId {
        self.layer
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
}

/// A stroke being drawn on the active layer of `base`.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeSession {
    base: Artwork,
    active: ActiveStroke,
}

impl Artwork {
    /// Starts a stroke on the active layer with its first sample.
    ///
    /// # Errors
    /// `LayerLocked` when the active layer is locked.
    pub fn begin_stroke(
        &self,
        style: StrokeStyle,
        first: StrokeSample,
    ) -> Result<StrokeSession, CoreError> {
        let layer = self.active();
        let index = self.editable_layer(layer)?;
        Ok(StrokeSession {
            base: self.clone(),
            active: ActiveStroke {
                layer,
                index,
                style,
                samples: vec![first],
            },
        })
    }
}

impl StrokeSession {
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

    /// Commits the stroke to its layer as one new version of the base artwork.
    #[must_use]
    pub fn finish(self) -> Applied {
        let mut next = self.base.state().clone();
        let id = next.ids.take_stroke();
        let active = self.active;
        let stroke = Stroke::new(id, active.style, Arc::from(active.samples));
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
