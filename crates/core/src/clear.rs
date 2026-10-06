//! Cut (S40, Procreate's): what is selected leaves the layer. A cut is a record among the
//! layer's edits, the selection's pixels cleared with a hard edge (as a transform clears what
//! it lifts) after the strokes the layer held then; nothing is baked, so undo is a version
//! switch and later strokes land on top. Cutting with nothing selected empties the layer's
//! paint. A copy made inside a selection clears the pixels outside it the same way (`copy.rs`).

use std::sync::Arc;

use crate::artwork::Artwork;
use crate::command::Applied;
use crate::error::CoreError;
use crate::layer_command::content;
use crate::region::{RegionEdit, RegionMask, check_mask};
use crate::stroke_list::StrokeList;

/// The pixels of `mask` made transparent, after the layer's first `after` strokes.
#[derive(Debug, Clone, PartialEq)]
pub struct ClearEdit {
    pub(crate) after: usize,
    pub(crate) mask: RegionMask,
}

impl ClearEdit {
    /// How many of the layer's strokes lie under it.
    #[must_use]
    pub const fn after(&self) -> usize {
        self.after
    }

    #[must_use]
    pub const fn mask(&self) -> &RegionMask {
        &self.mask
    }
}

impl Artwork {
    /// Cuts `clip` out of the active layer's paint (its mask stays as it is), or with no clip
    /// everything it holds.
    ///
    /// # Errors
    /// `LayerLocked` for a locked layer; `NothingToCopy` when its paint holds nothing;
    /// `InvalidValue` when the clip leaves the canvas.
    pub fn cut(&self, clip: Option<&RegionMask>) -> Result<Applied, CoreError> {
        let id = self.active();
        let index = self.editable_layer(id)?;
        let mut next = self.state().clone();
        let layer = &mut next.layers[index];
        if layer.strokes.is_empty() && layer.edits.is_empty() {
            return Err(CoreError::NothingToCopy);
        }
        match clip {
            Some(clip) => {
                check_mask(self.canvas(), clip)?;
                let mut edits = layer.edits.to_vec();
                edits.push(RegionEdit::Clear(ClearEdit {
                    after: layer.strokes.len(),
                    mask: clip.clone(),
                }));
                layer.edits = Arc::from(edits);
            }
            None => {
                layer.strokes = StrokeList::default();
                layer.edits = Arc::from([]);
            }
        }
        Ok(content(next))
    }
}
