//! Merge Down (S39, Procreate's): a layer goes into the layer below it as a nested record
//! (`nested.rs`), so the canvas shows what it showed. Allowed only where that holds by the
//! compositing rules (D50): a layer clipped to the layer below (its base) merges whatever its
//! mode, opacity and masks, as the clipping chain was that very composite; otherwise both must
//! composite by one mode that composes (Normal, Multiply or Screen: `a o (b o c) = (a o b) o c`),
//! so the lower layer's opacity and mask move into a nested record of their own when they are
//! not neutral and the merged layer takes full opacity; and the lower layer must not clip while
//! the upper does not (the upper would start to clip), nor may layers clip to the upper one
//! while it does not clip (they would start to clip to both). Hidden and locked layers, a layer
//! with nothing under it in its group and records nested too deep refuse (`Unmergeable`).

use std::sync::Arc;

use crate::artwork::Artwork;
use crate::blend::BlendMode;
use crate::command::Applied;
use crate::error::CoreError;
use crate::id::LayerId;
use crate::layer::Layer;
use crate::layer_command::content;
use crate::nested::{MAX_NESTING, NestedEdit, NestedLayer, nesting};
use crate::region::RegionEdit;

/// Why a layer cannot merge into the one below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unmergeable {
    /// It is the bottom of the stack or of its group.
    NothingBelow,
    /// It or the layer below is hidden: what shows would change.
    Hidden,
    /// It or the layer below is locked.
    Locked,
    /// It does not clip to the layer below and the two composite by different modes, or by
    /// Overlay, which does not compose.
    Modes,
    /// The layer below clips to another while it does not: it would start to clip.
    BelowClips,
    /// Layers clip to it while it does not clip: they would clip to both.
    ClippedTo,
    /// Its records would lie more than `MAX_NESTING` deep.
    TooDeep,
}

/// How the upper layer goes into the lower one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Merge {
    /// It clipped to the lower layer, its base: atop it.
    Atop,
    /// By the mode both share, over it.
    Over,
}

impl Artwork {
    /// Whether the layer `id` can merge into the layer below it now (Merge Down).
    ///
    /// # Errors
    /// Why not; `NothingBelow` for an unknown layer.
    pub fn mergeable(&self, id: LayerId) -> Result<(), Unmergeable> {
        let index = self.position(id).map_err(|_| Unmergeable::NothingBelow)?;
        self.merge_of(index).map(|_| ())
    }

    /// How the layer at `index` merges into the one below, or why it cannot.
    fn merge_of(&self, index: usize) -> Result<Merge, Unmergeable> {
        let layers = self.layers();
        let (Some(below), Some(upper)) = (index.checked_sub(1), layers.get(index)) else {
            return Err(Unmergeable::NothingBelow);
        };
        let lower = &layers[below];
        if lower.props.group != upper.props.group {
            return Err(Unmergeable::NothingBelow);
        }
        if !lower.props.visible || !upper.props.visible {
            return Err(Unmergeable::Hidden);
        }
        if lower.props.locked || upper.props.locked {
            return Err(Unmergeable::Locked);
        }
        let merge = if self.clips_at(index) && !self.clips_at(below) {
            Merge::Atop
        } else if self.clips_at(below) && !self.clips_at(index) {
            return Err(Unmergeable::BelowClips);
        } else if !self.clips_at(index) && self.clips_at(index + 1) {
            return Err(Unmergeable::ClippedTo);
        } else if lower.props.blend != upper.props.blend || upper.props.blend == BlendMode::Overlay
        {
            return Err(Unmergeable::Modes);
        } else {
            Merge::Over
        };
        let lower_depth = nesting(&lower.edits) + usize::from(nests_lower(lower, merge));
        let upper_depth = nesting(&upper.edits) + 1;
        if lower_depth.max(upper_depth) > MAX_NESTING {
            return Err(Unmergeable::TooDeep);
        }
        Ok(merge)
    }
}

/// Whether the lower layer's own content goes into a nested record too: when its enabled mask
/// (and, merged over, an opacity below 1) must apply to it alone.
fn nests_lower(lower: &Layer, merge: Merge) -> bool {
    let masked = lower.mask.as_ref().is_some_and(|mask| mask.enabled);
    masked || (merge == Merge::Over && lower.props.opacity < 1.0)
}

/// The layer `id` merged into the one below it, which becomes the active layer (its paint the
/// target) and keeps its id, name, group, switches and place.
pub(crate) fn merge_down(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    let index = artwork.position(id)?;
    let merge = artwork
        .merge_of(index)
        .map_err(|why| CoreError::Unmergeable { layer: id, why })?;
    let mut next = artwork.state().clone();
    let upper = next.layers.remove(index);
    let below = index - 1;
    let mut lower = next.layers[below].clone();
    if nests_lower(&lower, merge) {
        let mask = lower.mask.take_if(|mask| mask.enabled);
        let own = NestedLayer {
            id: next.ids.take_layer(),
            opacity: if merge == Merge::Over {
                lower.props.opacity
            } else {
                1.0
            },
            blend: BlendMode::Normal,
            clipped: false,
            strokes: std::mem::take(&mut lower.strokes),
            edits: std::mem::replace(&mut lower.edits, Arc::from([])),
            mask,
        };
        lower.edits = Arc::from([nested_edit(0, own)]);
        if merge == Merge::Over {
            lower.props.opacity = 1.0;
        }
    }
    let merged = NestedLayer {
        id: upper.id,
        opacity: upper.props.opacity,
        blend: upper.props.blend,
        clipped: merge == Merge::Atop,
        strokes: upper.strokes,
        edits: upper.edits,
        mask: upper.mask.filter(|mask| mask.enabled),
    };
    let mut edits = lower.edits.to_vec();
    edits.push(nested_edit(lower.strokes.len(), merged));
    lower.edits = Arc::from(edits);
    let lower_id = lower.id;
    next.layers[below] = lower;
    next.activate(lower_id);
    Ok(content(next))
}

fn nested_edit(after: usize, layer: NestedLayer) -> RegionEdit {
    RegionEdit::Nested(NestedEdit {
        after,
        layer: Arc::new(layer),
    })
}
