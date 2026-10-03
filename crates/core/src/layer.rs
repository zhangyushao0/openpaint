//! `Layer` and its properties. Layers change only through artwork commands and stroke sessions.

use std::sync::Arc;

use crate::error::CoreError;
use crate::id::LayerId;
use crate::region::RegionEdit;
use crate::stroke_list::StrokeList;
use crate::value::{LayerName, invalid, is_unit};

/// Layer properties the user toggles or slides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerProps {
    pub(crate) visible: bool,
    pub(crate) opacity: f32,
    pub(crate) locked: bool,
    pub(crate) alpha_locked: bool,
}

impl LayerProps {
    pub(crate) fn new(
        visible: bool,
        opacity: f32,
        locked: bool,
        alpha_locked: bool,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            visible,
            opacity: check_opacity(opacity)?,
            locked,
            alpha_locked,
        })
    }

    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    /// In `0..=1`.
    #[must_use]
    pub const fn opacity(&self) -> f32 {
        self.opacity
    }

    /// A locked layer's content cannot change; its properties and position still can.
    #[must_use]
    pub const fn locked(&self) -> bool {
        self.locked
    }

    /// Strokes begun while this is on paint only where the layer already has paint, keeping
    /// its alpha; the eraser does nothing on such a layer (Procreate's Alpha Lock).
    #[must_use]
    pub const fn alpha_locked(&self) -> bool {
        self.alpha_locked
    }
}

impl Default for LayerProps {
    /// Visible, fully opaque, unlocked, alpha not locked.
    fn default() -> Self {
        Self {
            visible: true,
            opacity: 1.0,
            locked: false,
            alpha_locked: false,
        }
    }
}

pub(crate) fn check_opacity(opacity: f32) -> Result<f32, CoreError> {
    if is_unit(opacity) {
        Ok(opacity)
    } else {
        Err(invalid("layer opacity"))
    }
}

/// One layer: identity, name, properties, strokes and region edits (fills). Clone is cheap
/// (shared strokes, edits and name).
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub(crate) id: LayerId,
    pub(crate) name: LayerName,
    pub(crate) props: LayerProps,
    pub(crate) strokes: StrokeList,
    /// Oldest first; each lies after its `after` strokes, so `after` never decreases.
    pub(crate) edits: Arc<[RegionEdit]>,
}

impl Layer {
    /// An empty layer with default properties.
    pub(crate) fn empty(id: LayerId) -> Self {
        Self {
            id,
            name: LayerName::numbered(id.get()),
            props: LayerProps::default(),
            strokes: StrokeList::default(),
            edits: Arc::from([]),
        }
    }

    /// The fills and transforms, oldest first. Drawing order: the edits with `after` 0, stroke
    /// 0, the edits with `after` 1, stroke 1, and so on.
    #[must_use]
    pub fn edits(&self) -> &[RegionEdit] {
        &self.edits
    }

    /// Heap bytes of the edits that `base` (the same layer in an older version) does not share.
    pub(crate) fn edit_bytes_since(&self, base: Option<&Self>) -> usize {
        let old = base.map_or(&[][..], |base| &base.edits[..]);
        if base.is_some_and(|base| Arc::ptr_eq(&self.edits, &base.edits)) {
            return 0;
        }
        let mut bytes = size_of_val::<[RegionEdit]>(&self.edits);
        for (index, edit) in self.edits.iter().enumerate() {
            bytes += edit.bytes_since(old.get(index));
        }
        bytes
    }

    #[must_use]
    pub const fn id(&self) -> LayerId {
        self.id
    }

    #[must_use]
    pub const fn name(&self) -> &LayerName {
        &self.name
    }

    #[must_use]
    pub const fn props(&self) -> &LayerProps {
        &self.props
    }

    /// Oldest first.
    #[must_use]
    pub const fn strokes(&self) -> &StrokeList {
        &self.strokes
    }
}
