//! An adjustment as a scene op (S28, D54): it reaches every pixel of its clip, or with no clip
//! every pixel of the layer, so its bounds are the clip's rectangle or unbounded; the curves'
//! tables go into the mask words. A filter (S33, D59) is such an op too, whose result the
//! target's arena holds (`filter.rs`).

use std::sync::Arc;

use openpaint_core::{AdjustEdit, Adjustment, COORD_LIMIT};

use crate::edits::mask_bounds;
use crate::error::RenderError;
use crate::filter::{FilterItem, FilterKey};
use crate::geom::Bounds;
use crate::protocol::{AdjustOn, Op, table_words};
use crate::scene::{Item, Scene};

impl Scene {
    pub(crate) fn push_adjust(
        &mut self,
        adjust: &AdjustEdit,
        on_mask: bool,
    ) -> Result<(), RenderError> {
        let on = match (on_mask, adjust.inverted()) {
            (false, _) => AdjustOn::Paint,
            (true, false) => AdjustOn::Mask,
            (true, true) => AdjustOn::InvertedMask,
        };
        let clip = adjust.clip().map(|clip| self.clip(clip)).transpose()?;
        let clip_value = clip.unwrap_or(0);
        let op = match adjust.adjustment() {
            Adjustment::Hsb(hsb) => {
                Op::adjust_hsb([hsb.hue, hsb.saturation, hsb.brightness], on, clip_value)
            }
            Adjustment::Curves(curves) => {
                let at = self.push_words(&table_words(&curves.luts()))?;
                Op::adjust_curves(at, on, clip_value)
            }
            Adjustment::Blur(_) | Adjustment::Sharpen(_) => {
                return Err(RenderError::Invalid("a filter is no point operation"));
            }
        };
        self.items.push(Item::Fill {
            op,
            bounds: adjust_bounds(adjust),
        });
        Ok(())
    }

    /// The filter `key` names, whose layer's items start at `start`; its op names its block once
    /// `place_bases` has run. Shown again in content a selection's record moved, it takes its
    /// first place's key and items.
    pub(crate) fn push_filter(&mut self, key: FilterKey, start: usize) -> Result<(), RenderError> {
        let Some(adjust) = key.edit().cloned() else {
            return Err(RenderError::Invalid("a filter item holds no filter"));
        };
        let clip = adjust.clip().map(|clip| self.clip(clip)).transpose()?;
        let first = self
            .filters
            .iter()
            .find(|item| item.key.place() == key.place());
        let (key, before) = match first {
            Some(first) => (Arc::clone(&first.key), first.before.clone()),
            None => (Arc::new(key), start..self.items.len()),
        };
        let item = FilterItem {
            key,
            before,
            item: self.items.len(),
            clip: clip.unwrap_or(0),
        };
        self.items.push(Item::Fill {
            op: Op::filter(0, item.clip, self.canvas),
            bounds: adjust_bounds(&adjust),
        });
        self.filters.push(item);
        Ok(())
    }

    /// Appends `words` to the mask words; returns where they start.
    fn push_words(&mut self, words: &[u32]) -> Result<u32, RenderError> {
        let at = u32::try_from(self.masks.len())
            .ok()
            .filter(|at| *at < u32::MAX)
            .ok_or(RenderError::Invalid("the masks exceed 2^32 words"))?;
        self.masks.extend_from_slice(words);
        Ok(at)
    }
}

/// The clip's rectangle, or unbounded.
fn adjust_bounds(adjust: &AdjustEdit) -> Bounds {
    adjust.clip().map_or(
        Bounds([-COORD_LIMIT, -COORD_LIMIT, COORD_LIMIT, COORD_LIMIT]),
        mask_bounds,
    )
}
