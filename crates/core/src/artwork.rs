//! `Artwork`: an immutable snapshot of a whole painting. Edits return a new version.

use std::sync::Arc;

use crate::error::CoreError;
use crate::group::LayerGroup;
use crate::id::{GroupId, IdCounters, LayerId};
use crate::layer::Layer;
use crate::mask::LayerMask;
use crate::value::CanvasSize;

/// Most layers one artwork may hold.
pub const MAX_LAYERS: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ArtworkState {
    pub(crate) canvas: CanvasSize,
    /// Bottom to top; never empty.
    pub(crate) layers: Vec<Layer>,
    /// Oldest first; each holds at least one layer, its layers next to each other
    /// (`group::check_groups`).
    pub(crate) groups: Vec<LayerGroup>,
    /// Always the id of one of `layers`.
    pub(crate) active: LayerId,
    /// The active layer's mask is the target of strokes and fills; only while it has one and
    /// it is enabled.
    pub(crate) on_mask: bool,
    pub(crate) ids: IdCounters,
}

impl ArtworkState {
    /// Makes `id` the active layer, its paint the target.
    pub(crate) fn activate(&mut self, id: LayerId) {
        self.active = id;
        self.on_mask = false;
    }
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
            groups: Vec::new(),
            on_mask: false,
            ids,
        })
    }

    /// The groups, oldest first.
    #[must_use]
    pub fn groups(&self) -> &[LayerGroup] {
        &self.0.groups
    }

    #[must_use]
    pub fn group(&self, id: GroupId) -> Option<&LayerGroup> {
        self.0.groups.iter().find(|group| group.id == id)
    }

    /// Whether the layer shows: its own eye and its group's are open. False for an unknown id.
    #[must_use]
    pub fn shown(&self, id: LayerId) -> bool {
        self.layer(id).is_some_and(|layer| {
            let group = layer.props.group.and_then(|group| self.group(group));
            layer.props.visible && group.is_none_or(LayerGroup::visible)
        })
    }

    /// Whether the layer at `index` (from the bottom) clips to a base under it: it is a
    /// Clipping Mask and the layer right under it lies in the same group (or in none, as it
    /// does). Its base is then the nearest layer under it that does not clip. False out of range.
    #[must_use]
    pub fn clips_at(&self, index: usize) -> bool {
        let layers = &self.0.layers;
        index > 0
            && index < layers.len()
            && layers[index].props.clipping
            && layers[index - 1].props.group == layers[index].props.group
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
        let mut bytes = size_of::<ArtworkState>()
            + size_of_val::<[Layer]>(layers)
            + size_of_val::<[LayerGroup]>(&self.0.groups);
        for (index, layer) in layers.iter().enumerate() {
            let before = match base.0.layers.get(index) {
                Some(same) if same.id == layer.id => Some(same),
                _ => base.layer(layer.id),
            };
            bytes += layer.bytes_since(before);
        }
        bytes
    }

    /// Whether new strokes and fills go to the active layer's mask rather than its paint (S24).
    /// Only ever true while the active layer has a mask.
    #[must_use]
    pub fn on_mask(&self) -> bool {
        self.0.on_mask
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

    /// The positions of the clipping chain holding the layer at `index`: its base and every
    /// layer clipping to it (just `index` for a layer in no chain).
    pub(crate) fn chain(&self, index: usize) -> std::ops::Range<usize> {
        let mut start = index;
        while self.clips_at(start) {
            start -= 1;
        }
        let mut end = index + 1;
        while self.clips_at(end) {
            end += 1;
        }
        start..end
    }

    /// The layer's position, provided its content may change.
    pub(crate) fn editable_layer(&self, id: LayerId) -> Result<usize, CoreError> {
        let index = self.position(id)?;
        if self.0.layers[index].props.locked {
            return Err(CoreError::LayerLocked(id));
        }
        Ok(index)
    }

    /// The active layer's position and, when its mask is the target, that mask, provided what
    /// new strokes and fills go to may change: `MaskOff` for a disabled mask.
    pub(crate) fn target(&self) -> Result<(usize, Option<&LayerMask>), CoreError> {
        let id = self.0.active;
        let index = self.editable_layer(id)?;
        let mask = self.0.layers[index]
            .mask
            .as_ref()
            .filter(|_| self.0.on_mask);
        if mask.is_some_and(|mask| !mask.enabled) {
            return Err(CoreError::MaskOff(id));
        }
        Ok((index, mask))
    }
}
