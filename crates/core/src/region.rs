//! Region edits: a set of canvas pixels (`RegionMask`) and what was done to them, at their place
//! between the layer's strokes. A fill (ColorDrop) paints one colour over every pixel of its
//! mask, with a hard edge; a transform (`transform.rs`) moves pixels; an adjustment
//! (`adjust.rs`) changes their colours.

use std::sync::Arc;

use crate::adjust::AdjustEdit;
use crate::artwork::Artwork;
use crate::command::{Applied, Change};
use crate::error::CoreError;
use crate::transform::TransformEdit;
use crate::value::{CanvasSize, Color, invalid};

/// A non-empty set of canvas pixels inside a bounding rectangle. Rows are bit-packed: pixel
/// `origin + (x, y)` is bit `x % 32` of word `y * stride + x / 32`. Clone is O(1).
#[derive(Debug, Clone)]
pub struct RegionMask {
    pub(crate) origin: [u32; 2],
    pub(crate) size: [u32; 2],
    pub(crate) words: Arc<[u32]>,
}

impl RegionMask {
    /// The pixels of a `canvas`-sized picture, row by row, for which `inside` is true, in their
    /// tight bounding rectangle.
    ///
    /// # Errors
    /// `InvalidValue` when `inside` is not one value per canvas pixel or holds no pixel.
    pub fn from_pixels(canvas: CanvasSize, inside: &[bool]) -> Result<Self, CoreError> {
        let (width, height) = (canvas.width() as usize, canvas.height() as usize);
        if inside.len() != width * height {
            return Err(invalid("region mask"));
        }
        let mut min = [usize::MAX; 2];
        let mut max = [0usize; 2];
        for (row, line) in inside.chunks(width).enumerate() {
            let first = line.iter().position(|&pixel| pixel);
            let last = line.iter().rposition(|&pixel| pixel);
            if let (Some(first), Some(last)) = (first, last) {
                min = [min[0].min(first), min[1].min(row)];
                max = [max[0].max(last + 1), row + 1];
            }
        }
        if min[0] == usize::MAX {
            return Err(invalid("region mask"));
        }
        let size = [max[0] - min[0], max[1] - min[1]];
        let stride = size[0].div_ceil(32);
        let mut words = vec![0u32; stride * size[1]];
        for y in 0..size[1] {
            let line = &inside[(min[1] + y) * width + min[0]..][..size[0]];
            for (x, _) in line.iter().enumerate().filter(|(_, pixel)| **pixel) {
                words[y * stride + x / 32] |= 1 << (x % 32);
            }
        }
        let side = |value: usize| u32::try_from(value).map_err(|_| invalid("region mask"));
        Ok(Self {
            origin: [side(min[0])?, side(min[1])?],
            size: [side(size[0])?, side(size[1])?],
            words: Arc::from(words),
        })
    }

    /// A mask from stored runs: over the rectangle at `origin` of `size` pixels, row by row,
    /// alternately pixels outside and inside, starting outside (the first run may be 0).
    ///
    /// # Errors
    /// `InvalidValue` when the rectangle leaves `canvas` or is empty, the runs do not add up to
    /// its pixels, or no pixel is inside.
    pub fn from_runs(
        canvas: CanvasSize,
        origin: [u32; 2],
        size: [u32; 2],
        runs: &[u32],
    ) -> Result<Self, CoreError> {
        let fits = |at: u32, side: u32, limit: u32| {
            side > 0 && at.checked_add(side).is_some_and(|end| end <= limit)
        };
        if !fits(origin[0], size[0], canvas.width()) || !fits(origin[1], size[1], canvas.height()) {
            return Err(invalid("region mask"));
        }
        let (width, total) = (size[0] as usize, size[0] as usize * size[1] as usize);
        let stride = width.div_ceil(32);
        let mut words = vec![0u32; stride * size[1] as usize];
        let (mut at, mut any) = (0usize, false);
        for (index, &run) in runs.iter().enumerate() {
            let end = at.checked_add(run as usize).filter(|&end| end <= total);
            let end = end.ok_or(invalid("region mask"))?;
            if index % 2 == 1 {
                any |= run > 0;
                for pixel in at..end {
                    let (x, y) = (pixel % width, pixel / width);
                    words[y * stride + x / 32] |= 1 << (x % 32);
                }
            }
            at = end;
        }
        if at != total || !any {
            return Err(invalid("region mask"));
        }
        Ok(Self {
            origin,
            size,
            words: Arc::from(words),
        })
    }

    /// The mask as runs for storage, as `from_runs` reads them.
    #[must_use]
    pub fn runs(&self) -> Vec<u32> {
        let mut runs = Vec::new();
        let (mut inside, mut length) = (false, 0u32);
        for y in 0..self.size[1] {
            for x in 0..self.size[0] {
                if self.bit(x, y) != inside {
                    runs.push(length);
                    (inside, length) = (!inside, 0);
                }
                length += 1;
            }
        }
        runs.push(length);
        runs
    }

    /// Top-left pixel of the bounding rectangle.
    #[must_use]
    pub const fn origin(&self) -> [u32; 2] {
        self.origin
    }

    /// Width and height of the bounding rectangle, each at least 1.
    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Words per row.
    #[must_use]
    pub const fn stride(&self) -> u32 {
        self.size[0].div_ceil(32)
    }

    /// The bit-packed rows.
    #[must_use]
    pub fn words(&self) -> &[u32] {
        &self.words
    }

    /// Whether canvas pixel `[x, y]` is in the mask.
    #[must_use]
    pub fn contains(&self, [x, y]: [u32; 2]) -> bool {
        let [ox, oy] = self.origin;
        x >= ox
            && y >= oy
            && x - ox < self.size[0]
            && y - oy < self.size[1]
            && self.bit(x - ox, y - oy)
    }

    /// True when both share one copy of their pixels.
    #[must_use]
    pub fn shares(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.words, &other.words)
    }

    pub(crate) fn bit(&self, x: u32, y: u32) -> bool {
        let word = self.words[(y * self.stride() + x / 32) as usize];
        word >> (x % 32) & 1 == 1
    }
}

impl PartialEq for RegionMask {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.size == other.size
            && (self.shares(other) || self.words == other.words)
    }
}

/// What was done to a layer's pixels, at its place among the layer's strokes: a fill, a
/// transform or an adjustment.
#[derive(Debug, Clone, PartialEq)]
pub enum RegionEdit {
    Fill(FillEdit),
    Transform(TransformEdit),
    Adjust(AdjustEdit),
}

impl RegionEdit {
    /// How many of the layer's strokes lie under the edit (oldest first).
    #[must_use]
    pub const fn after(&self) -> usize {
        match self {
            Self::Fill(fill) => fill.after,
            Self::Transform(transform) => transform.after,
            Self::Adjust(adjust) => adjust.after,
        }
    }

    #[must_use]
    pub const fn fill(&self) -> Option<&FillEdit> {
        match self {
            Self::Fill(fill) => Some(fill),
            _ => None,
        }
    }

    #[must_use]
    pub const fn transform(&self) -> Option<&TransformEdit> {
        match self {
            Self::Transform(transform) => Some(transform),
            _ => None,
        }
    }

    #[must_use]
    pub const fn adjust(&self) -> Option<&AdjustEdit> {
        match self {
            Self::Adjust(adjust) => Some(adjust),
            _ => None,
        }
    }

    /// The same edit seen from two versions: its pixels shared, its place and values equal.
    /// O(1): never compares pixels.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fill(a), Self::Fill(b)) => {
                a.mask.shares(&b.mask)
                    && a.after == b.after
                    && a.color == b.color
                    && a.alpha_locked == b.alpha_locked
            }
            (Self::Transform(a), Self::Transform(b)) => a.same(b),
            (Self::Adjust(a), Self::Adjust(b)) => a.same(b),
            _ => false,
        }
    }

    /// Heap bytes of its masks and pixels that `old` (the edit at its index in an older
    /// version) does not share.
    pub(crate) fn bytes_since(&self, old: Option<&Self>) -> usize {
        let mask = |mask: &RegionMask, old: Option<&RegionMask>| {
            if old.is_some_and(|old| old.shares(mask)) {
                0
            } else {
                size_of_val(mask.words())
            }
        };
        match self {
            Self::Fill(fill) => mask(&fill.mask, old.and_then(Self::fill).map(|o| &o.mask)),
            Self::Transform(transform) => {
                let old = old.and_then(Self::transform);
                let lift = transform
                    .lift
                    .as_ref()
                    .map_or(0, |lift| mask(lift, old.and_then(TransformEdit::lift)));
                let shared = old.is_some_and(|old| old.patch.shares(&transform.patch));
                lift + if shared {
                    0
                } else {
                    transform.patch.pixels().len()
                }
            }
            Self::Adjust(adjust) => {
                let old = old.and_then(Self::adjust).and_then(AdjustEdit::clip);
                adjust.clip.as_ref().map_or(0, |clip| mask(clip, old))
            }
        }
    }
}

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

pub(crate) fn check_mask(canvas: CanvasSize, mask: &RegionMask) -> Result<(), CoreError> {
    let [x, y] = mask.origin;
    let [w, h] = mask.size;
    let fits =
        |at: u32, side: u32, limit: u32| at.checked_add(side).is_some_and(|end| end <= limit);
    if fits(x, w, canvas.width()) && fits(y, h, canvas.height()) {
        Ok(())
    } else {
        Err(invalid("region mask"))
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
        Ok(Applied {
            artwork: push_edit(self, index, RegionEdit::Fill(edit)),
            change: Change::Content,
        })
    }
}

/// `artwork` with `edit` added to the edits of its layer at `index`, or of that layer's mask
/// when the mask is the target.
pub(crate) fn push_edit(artwork: &Artwork, index: usize, edit: RegionEdit) -> Artwork {
    let mut next = artwork.state().clone();
    let on_mask = next.on_mask;
    let layer = &mut next.layers[index];
    let edits = match layer.mask.as_mut().filter(|_| on_mask) {
        Some(mask) => &mut mask.edits,
        None => &mut layer.edits,
    };
    let mut grown = edits.to_vec();
    grown.push(edit);
    *edits = Arc::from(grown);
    Artwork::from_state(next)
}
