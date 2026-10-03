//! `Artwork`: an immutable snapshot of a whole painting. Edits return a new version.

use std::sync::Arc;

use crate::error::CoreError;
use crate::id::{IdCounters, LayerId};
use crate::layer::Layer;
use crate::value::CanvasSize;

/// Most layers one artwork may hold.
pub const MAX_LAYERS: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ArtworkState {
    pub(crate) canvas: CanvasSize,
    /// Bottom to top; never empty.
    pub(crate) layers: Vec<Layer>,
    /// Always the id of one of `layers`.
    pub(crate) active: LayerId,
    pub(crate) ids: IdCounters,
}

/// A painting as a value: clone is O(1), equal versions compare equal, and nothing mutates it.
/// New versions come from `apply`, from finishing a `StrokeSession` and from `from_storage`.
#[derive(Debug, Clone, PartialEq)]
pub struct Artwork(Arc<ArtworkState>);

impl Artwork {
    /// A blank artwork with one empty layer, "Layer 1", which is active.
    #[must_use]
    pub fn new(canvas: CanvasSize) -> Self {
        let mut ids = IdCounters {
            next_layer: 1,
            next_stroke: 1,
        };
        let first = Layer::empty(ids.take_layer());
        Self::from_state(ArtworkState {
            canvas,
            active: first.id,
            layers: vec![first],
            ids,
        })
    }

    #[must_use]
    pub fn canvas(&self) -> CanvasSize {
        self.0.canvas
    }

    /// Bottom to top; never empty.
    #[must_use]
    pub fn layers(&self) -> &[Layer] {
        &self.0.layers
    }

    #[must_use]
    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.0.layers.iter().find(|layer| layer.id == id)
    }

    /// The layer new strokes go to; always one of `layers()`.
    #[must_use]
    pub fn active(&self) -> LayerId {
        self.0.active
    }

    /// The id the next new layer gets; ids are never reused. For storage.
    #[must_use]
    pub fn next_layer_id(&self) -> u64 {
        self.0.ids.next_layer
    }

    /// The id the next finished stroke gets; ids are never reused. For storage.
    #[must_use]
    pub fn next_stroke_id(&self) -> u64 {
        self.0.ids.next_stroke
    }

    /// True when both handles share one version, which means nothing changed between them.
    /// Equal content in different versions compares equal with `==` but is not the same version.
    #[must_use]
    pub fn same_version(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Heap bytes this version holds that `base` does not share, for history budgets: the
    /// state and its layer table, and per layer the stroke chunks and samples that `base`'s
    /// layer of the same id does not share, and the fill masks it does not share. Unchanged
    /// layers cost O(1), changed ones O(chunks + edits), never O(strokes). Names are not
    /// counted.
    #[must_use]
    pub fn bytes_since(&self, base: &Self) -> usize {
        if self.same_version(base) {
            return 0;
        }
        let layers = &self.0.layers;
        let mut bytes = size_of::<ArtworkState>() + size_of_val::<[Layer]>(layers);
        for (index, layer) in layers.iter().enumerate() {
            let before = match base.0.layers.get(index) {
                Some(same) if same.id == layer.id => Some(same),
                _ => base.layer(layer.id),
            };
            bytes += layer
                .strokes
                .bytes_since(before.map(|before| &before.strokes));
            bytes += layer.edit_bytes_since(before);
        }
        bytes
    }

    pub(crate) fn from_state(state: ArtworkState) -> Self {
        Self(Arc::new(state))
    }

    pub(crate) fn state(&self) -> &ArtworkState {
        &self.0
    }

    /// The layer's position from the bottom.
    pub(crate) fn position(&self, id: LayerId) -> Result<usize, CoreError> {
        self.0
            .layers
            .iter()
            .position(|layer| layer.id == id)
            .ok_or(CoreError::UnknownLayer(id))
    }

    /// The layer's position, provided its content may change.
    pub(crate) fn editable_layer(&self, id: LayerId) -> Result<usize, CoreError> {
        let index = self.position(id)?;
        if self.0.layers[index].props.locked {
            return Err(CoreError::LayerLocked(id));
        }
        Ok(index)
    }
}
