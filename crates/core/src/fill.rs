//! Fill (ColorDrop): one colour over every pixel of a region, with a hard edge, at its place
//! among the layer's strokes.

use crate::artwork::Artwork;
use crate::command::Applied;
use crate::error::CoreError;
use crate::region::{RegionEdit, RegionMask, check_mask, push_edit};
use crate::value::Color;

/// A fill on a layer: `color` over every pixel of `mask`, painted after the layer's first
/// `after` strokes. With `alpha_locked` (the layer's Alpha Lock when it was made) the colour
/// changes only where the layer already has paint, keeping its alpha.
#[derive(Debug, Clone, PartialEq)]
pub struct FillEdit {
    pub(crate) after: usize,
    pub(crate) mask: RegionMask,
    pub(crate) color: Color,
    pub(crate) alpha_locked: bool,
}

impl FillEdit {
    /// How many of the layer's strokes lie under the fill (oldest first).
    #[must_use]
    pub const fn after(&self) -> usize {
        self.after
    }

    #[must_use]
    pub const fn mask(&self) -> &RegionMask {
        &self.mask
    }

    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    #[must_use]
    pub const fn alpha_locked(&self) -> bool {
        self.alpha_locked
    }
}

impl Artwork {
    /// Fills `mask` with `color` on the active layer, above its strokes so far (ColorDrop). On
    /// a layer with Alpha Lock on, the fill keeps the layer's alpha. When the layer's mask is
    /// the target the fill lands there, in the colour's grey.
    ///
    /// # Errors
    /// `LayerLocked` when the active layer is locked; `MaskOff` when its mask is the target
    /// and disabled; `InvalidValue` when the mask leaves the canvas.
    pub fn fill_region(&self, mask: RegionMask, color: Color) -> Result<Applied, CoreError> {
        let (index, target) = self.target()?;
        check_mask(self.canvas(), &mask)?;
        let layer = &self.state().layers[index];
        let edit = match target {
            Some(target) => FillEdit {
                after: target.strokes.len(),
                mask,
                color: target.grey(color),
                alpha_locked: false,
            },
            None => FillEdit {
                after: layer.strokes.len(),
                mask,
                color,
                alpha_locked: layer.props.alpha_locked,
            },
        };
        Ok(push_edit(self, index, RegionEdit::Fill(edit)))
    }
}
