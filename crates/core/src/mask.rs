//! Layer masks (S24): a layer's grey content that shows or hides the layer's paint. White
//! reveals, black hides, grey shows a share; a new mask is white everywhere, so nothing changes
//! until it is painted on. A mask holds strokes and fills like a layer, painted in neutral grey
//! over its white (`LayerMask`); the brush paints the current colour's grey, the eraser white.
//! Inverting flips a flag; strokes and fills made while it is on store the opposite grey, so
//! each still shows as it was drawn and a second invert brings everything back (an inverted
//! grey over an inverted mask is the inverse of the grey over the mask).

use std::sync::Arc;

use crate::region::RegionEdit;
use crate::stroke::{StrokeStyle, StrokeTool};
use crate::stroke_list::StrokeList;
use crate::value::Color;

/// A layer's mask: whether it applies, whether it is inverted, and what was painted on it.
/// Clone is cheap (shared strokes and edits).
#[derive(Debug, Clone, PartialEq)]
pub struct LayerMask {
    pub(crate) enabled: bool,
    pub(crate) inverted: bool,
    /// Neutral grey paint over white, or smudges; never under Alpha Lock.
    pub(crate) strokes: StrokeList,
    /// Fills only, in neutral grey; each lies after its `after` strokes.
    pub(crate) edits: Arc<[RegionEdit]>,
}

impl LayerMask {
    /// White everywhere, applied, not inverted: the layer shows as it did.
    pub(crate) fn white() -> Self {
        Self {
            enabled: true,
            inverted: false,
            strokes: StrokeList::default(),
            edits: Arc::from([]),
        }
    }

    /// Whether the mask applies; a disabled mask keeps its content and shows the whole layer.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether what the strokes and fills leave is inverted (black for white) when it applies.
    #[must_use]
    pub const fn inverted(&self) -> bool {
        self.inverted
    }

    /// Oldest first, over a white mask.
    #[must_use]
    pub const fn strokes(&self) -> &StrokeList {
        &self.strokes
    }

    /// The fills, oldest first, at their places among the strokes as a layer's edits are.
    #[must_use]
    pub fn edits(&self) -> &[RegionEdit] {
        &self.edits
    }

    /// Whether `other` holds this very grey, shared (O(1)), inverted alike: the mask then shows
    /// the same, whether on or off.
    #[must_use]
    pub fn same_grey(&self, other: &Self) -> bool {
        self.inverted == other.inverted
            && self.strokes.same(&other.strokes)
            && Arc::ptr_eq(&self.edits, &other.edits)
    }

    /// Nothing painted and not inverted: it shows the whole layer.
    #[must_use]
    pub fn is_white(&self) -> bool {
        !self.inverted && self.strokes.is_empty() && self.edits.is_empty()
    }

    /// The grey a stroke or fill drawn now with `color` stores on this mask.
    pub(crate) fn grey(&self, color: Color) -> Color {
        let grey = grey_of(color);
        if self.inverted { flip(grey) } else { grey }
    }

    /// `style` as it paints on this mask: the brush lays the colour's grey, the eraser lays
    /// white (it reveals), the smudge moves the mask's own grey.
    pub(crate) fn style(&self, style: StrokeStyle) -> StrokeStyle {
        let white = if self.inverted {
            Color::BLACK
        } else {
            Color::WHITE
        };
        match style.tool() {
            StrokeTool::Paint => {
                let grey = self.grey(style.color());
                style.with_color(grey)
            }
            StrokeTool::Erase => style.with_color(white).with_tool(StrokeTool::Paint),
            StrokeTool::Smudge => style,
        }
    }
}

/// The colour's grey on a mask: its luma (Rec. 601 weights on the sRGB-encoded channels),
/// rounded to 8 bits, so black hides, white reveals and a colour hides by how dark it is.
#[must_use]
pub fn grey_of(color: Color) -> Color {
    let luma = 0.299 * color.r() + 0.587 * color.g() + 0.114 * color.b();
    let level = (luma.clamp(0.0, 1.0) * 255.0).round() / 255.0;
    Color::neutral(level)
}

fn flip(grey: Color) -> Color {
    Color::neutral((255.0 - (grey.r() * 255.0).round()) / 255.0)
}

/// True for a colour a mask can hold: the three channels equal.
pub(crate) fn is_neutral(color: Color) -> bool {
    color.r() == color.g() && color.g() == color.b()
}
