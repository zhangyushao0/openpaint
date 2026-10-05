//! `Layer` and its properties. Layers change only through artwork commands and stroke sessions.

use std::sync::Arc;

use crate::blend::BlendMode;
use crate::error::CoreError;
use crate::id::{GroupId, IdCounters, LayerId};
use crate::mask::LayerMask;
use crate::region::RegionEdit;
use crate::stroke_list::StrokeList;
use crate::value::{LayerName, invalid, is_unit};

/// Layer properties the user toggles or slides, and the group the layer belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerProps {
    pub(crate) visible: bool,
    pub(crate) opacity: f32,
    pub(crate) locked: bool,
    pub(crate) alpha_locked: bool,
    pub(crate) clipping: bool,
    pub(crate) blend: BlendMode,
    pub(crate) group: Option<GroupId>,
}

impl LayerProps {
    pub(crate) fn new(
        (visible, opacity): (bool, f32),
        (locked, alpha_locked): (bool, bool),
        (clipping, blend): (bool, BlendMode),
        group: Option<GroupId>,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            visible,
            opacity: check_opacity(opacity)?,
            locked,
            alpha_locked,
            clipping,
            blend,
            group,
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

    /// Clipping Mask: the layer shows only where the nearest layer below it in the same group
    /// that does not clip (its base) has paint. A clipping layer with no such layer under it in
    /// its group (the bottom of the stack or of a group) is a base itself
    /// (`Artwork::clips_at`).
    #[must_use]
    pub const fn clipping(&self) -> bool {
        self.clipping
    }

    #[must_use]
    pub const fn blend(&self) -> BlendMode {
        self.blend
    }

    /// The group the layer lies in, if any.
    #[must_use]
    pub const fn group(&self) -> Option<GroupId> {
        self.group
    }
}

impl Default for LayerProps {
    /// Visible, fully opaque, unlocked, alpha not locked, not clipping, Normal, in no group.
    fn default() -> Self {
        Self {
            visible: true,
            opacity: 1.0,
            locked: false,
            alpha_locked: false,
            clipping: false,
            blend: BlendMode::Normal,
            group: None,
        }
    }
}

/// Heap bytes of `edits` that `base` (the same list in an older version) does not share.
fn edit_bytes_since(edits: &Arc<[RegionEdit]>, base: Option<&Arc<[RegionEdit]>>) -> usize {
    if base.is_some_and(|base| Arc::ptr_eq(edits, base)) {
        return 0;
    }
    let old = base.map_or(&[][..], |base| &base[..]);
    let mut bytes = size_of_val::<[RegionEdit]>(edits);
    for (index, edit) in edits.iter().enumerate() {
        bytes += edit.bytes_since(old.get(index));
    }
    bytes
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
    pub(crate) mask: Option<LayerMask>,
}

impl Layer {
    /// An empty layer with default properties and no mask.
    pub(crate) fn empty(id: LayerId) -> Self {
        Self {
            id,
            name: LayerName::numbered(id.get()),
            props: LayerProps::default(),
            strokes: StrokeList::default(),
            edits: Arc::from([]),
            mask: None,
        }
    }

    /// The layer's mask, if it has one (S24).
    #[must_use]
    pub const fn mask(&self) -> Option<&LayerMask> {
        self.mask.as_ref()
    }

    /// The fills and transforms, oldest first. Drawing order: the edits with `after` 0, stroke
    /// 0, the edits with `after` 1, stroke 1, and so on.
    #[must_use]
    pub fn edits(&self) -> &[RegionEdit] {
        &self.edits
    }

    /// Whether `other` holds this very paint, shared (O(1)): what the layer shows alone, without
    /// its mask and properties, is then the same. A layer drawn on, edited or reloaded is not.
    #[must_use]
    pub fn same_paint(&self, other: &Self) -> bool {
        self.strokes.same(&other.strokes) && Arc::ptr_eq(&self.edits, &other.edits)
    }

    /// A copy of the layer under `id`, its strokes and its mask's renumbered from `ids`.
    pub(crate) fn duplicate(&self, id: LayerId, ids: &mut IdCounters) -> Self {
        let mut copy = self.clone();
        copy.id = id;
        copy.strokes = self.strokes.renumbered(ids);
        if let Some(mask) = &mut copy.mask {
            mask.strokes = mask.strokes.renumbered(ids);
        }
        copy
    }

    /// Heap bytes of the strokes and edits, its mask's included, that `base` (the same layer in
    /// an older version) does not share.
    pub(crate) fn bytes_since(&self, base: Option<&Self>) -> usize {
        let mut bytes = self.strokes.bytes_since(base.map(|base| &base.strokes))
            + edit_bytes_since(&self.edits, base.map(|base| &base.edits));
        if let Some(mask) = &self.mask {
            let old = base.and_then(|base| base.mask.as_ref());
            bytes += mask.strokes.bytes_since(old.map(|old| &old.strokes))
                + edit_bytes_since(&mask.edits, old.map(|old| &old.edits));
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
