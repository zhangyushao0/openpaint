//! Transform (Procreate's Transform, draweave's behaviour): a layer's content, or the part of it
//! inside a selection, lifted as pixels and placed again by an affine map. The pixels were
//! taken from the layer as it was (`Patch`); the edit clears what it lifted and draws the patch
//! through its `Placement`, at its place among the layer's strokes (DECISIONS D45).

use std::sync::Arc;

use crate::artwork::Artwork;
use crate::command::{Applied, Change};
use crate::error::CoreError;
use crate::region::{RegionEdit, RegionMask, check_mask, push_edit};
use crate::value::{COORD_LIMIT, CanvasSize, invalid};

/// The largest a placement may scale or shrink content: `1 / MAX_SCALE..=MAX_SCALE` per axis.
pub const MAX_SCALE: f32 = 100.0;

/// An affine map of canvas pixels: a point `p` of the content as it was lands on
/// `matrix * p + translation`. Never collapses content (its determinant stays well away from 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Row major: `[a, b, c, d]` maps `(x, y)` to `(a x + b y, c x + d y)`.
    matrix: [f32; 4],
    translation: [f32; 2],
}

impl Placement {
    /// Everything stays where it was.
    pub const IDENTITY: Self = Self {
        matrix: [1.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0],
    };

    /// `None` unless every value is finite, the translation stays within `COORD_LIMIT` and each
    /// axis scales by `1 / MAX_SCALE..=MAX_SCALE`.
    #[must_use]
    pub fn new(matrix: [f32; 4], translation: [f32; 2]) -> Option<Self> {
        let finite = matrix.iter().chain(&translation).all(|v| v.is_finite());
        let [a, b, c, d] = matrix;
        let range = (1.0 / MAX_SCALE)..=MAX_SCALE;
        let axes = [a.hypot(c), b.hypot(d)];
        let det = (a * d - b * c).abs();
        let valid = finite
            && translation.iter().all(|v| v.abs() <= COORD_LIMIT)
            && axes.iter().all(|axis| range.contains(axis))
            && det >= 1.0 / (MAX_SCALE * MAX_SCALE);
        valid.then_some(Self {
            matrix,
            translation,
        })
    }

    /// Moved by `offset` canvas pixels.
    #[must_use]
    pub fn moved(offset: [f32; 2]) -> Option<Self> {
        Self::new(Self::IDENTITY.matrix, offset)
    }

    /// Scaled by `scale`, turned by `rotation` (radians, clockwise on the canvas, y down) and
    /// mirrored left to right and top to bottom as `flip` says, about `pivot`, which stays.
    #[must_use]
    pub fn around(pivot: [f32; 2], scale: f32, rotation: f32, flip: [bool; 2]) -> Option<Self> {
        let (sin, cos) = rotation.sin_cos();
        let [sx, sy] = flip.map(|mirrored| if mirrored { -scale } else { scale });
        let matrix = [cos * sx, -sin * sy, sin * sx, cos * sy];
        let [x, y] = pivot;
        let turned = [matrix[0] * x + matrix[1] * y, matrix[2] * x + matrix[3] * y];
        Self::new(matrix, [x - turned[0], y - turned[1]])
    }

    /// This placement, then `next`.
    #[must_use]
    pub fn then(self, next: Self) -> Option<Self> {
        let [a, b, c, d] = self.matrix;
        let [na, nb, nc, nd] = next.matrix;
        let [tx, ty] = self.translation;
        Self::new(
            [
                na * a + nb * c,
                na * b + nb * d,
                nc * a + nd * c,
                nc * b + nd * d,
            ],
            [
                na * tx + nb * ty + next.translation[0],
                nc * tx + nd * ty + next.translation[1],
            ],
        )
    }

    /// Where the content's point `point` lands.
    #[must_use]
    pub fn apply(self, [x, y]: [f32; 2]) -> [f32; 2] {
        let [a, b, c, d] = self.matrix;
        [
            a * x + b * y + self.translation[0],
            c * x + d * y + self.translation[1],
        ]
    }

    /// The map back, as rows `[a, b, c, d, e, f]` of `p = (a x + b y + c, d x + e y + f)` for
    /// a placed point `(x, y)`: where on the content each placed pixel samples.
    #[must_use]
    pub fn inverse_rows(self) -> [f32; 6] {
        let [a, b, c, d] = self.matrix.map(f64::from);
        let [tx, ty] = self.translation.map(f64::from);
        let det = a * d - b * c;
        let [ia, ib, ic, id] = [d / det, -b / det, -c / det, a / det];
        [ia, ib, -(ia * tx + ib * ty), ic, id, -(ic * tx + id * ty)].map(|value| value as f32)
    }

    #[must_use]
    pub const fn matrix(self) -> [f32; 4] {
        self.matrix
    }

    #[must_use]
    pub const fn translation(self) -> [f32; 2] {
        self.translation
    }

    /// The scale of its first axis.
    #[must_use]
    pub fn scale(self) -> f32 {
        self.matrix[0].hypot(self.matrix[2])
    }
}

/// Pixels lifted from a layer: premultiplied 8-bit RGBA, rows top first, over the rectangle of
/// `size` pixels at canvas pixel `origin`. Clone is O(1).
#[derive(Debug, Clone)]
pub struct Patch {
    origin: [u32; 2],
    size: [u32; 2],
    pixels: Arc<[u8]>,
}

impl Patch {
    /// # Errors
    /// `InvalidValue` when the rectangle is empty or leaves `canvas`, or `pixels` is not four
    /// bytes per pixel of it.
    pub fn new(
        canvas: CanvasSize,
        origin: [u32; 2],
        size: [u32; 2],
        pixels: Vec<u8>,
    ) -> Result<Self, CoreError> {
        let fits = |at: u32, side: u32, limit: u32| {
            side > 0 && at.checked_add(side).is_some_and(|end| end <= limit)
        };
        let count = size[0] as usize * size[1] as usize * 4;
        if !fits(origin[0], size[0], canvas.width())
            || !fits(origin[1], size[1], canvas.height())
            || pixels.len() != count
        {
            return Err(invalid("patch"));
        }
        Ok(Self {
            origin,
            size,
            pixels: Arc::from(pixels),
        })
    }

    #[must_use]
    pub const fn origin(&self) -> [u32; 2] {
        self.origin
    }

    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Premultiplied RGBA, four bytes per pixel, rows top first.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// True when both share one copy of their pixels.
    #[must_use]
    pub fn shares(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.pixels, &other.pixels)
    }
}

impl PartialEq for Patch {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.size == other.size
            && (self.shares(other) || self.pixels == other.pixels)
    }
}

/// A transform on a layer, after its first `after` strokes: the pixels of `lift` (the whole
/// layer when `None`) are cleared, then `patch` (what was there) is drawn through `placement`.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformEdit {
    pub(crate) after: usize,
    pub(crate) lift: Option<RegionMask>,
    pub(crate) patch: Patch,
    pub(crate) placement: Placement,
}

impl TransformEdit {
    #[must_use]
    pub const fn after(&self) -> usize {
        self.after
    }

    /// The pixels taken away: a selection's, or `None` for everything the layer held so far.
    #[must_use]
    pub const fn lift(&self) -> Option<&RegionMask> {
        self.lift.as_ref()
    }

    #[must_use]
    pub const fn patch(&self) -> &Patch {
        &self.patch
    }

    #[must_use]
    pub const fn placement(&self) -> Placement {
        self.placement
    }

    /// The same transform seen from two versions: shared pixels, the same place and map.
    pub(crate) fn same(&self, other: &Self) -> bool {
        let lift = match (&self.lift, &other.lift) {
            (Some(a), Some(b)) => a.shares(b),
            (None, None) => true,
            _ => false,
        };
        lift && self.patch.shares(&other.patch)
            && self.after == other.after
            && self.placement == other.placement
    }
}

pub(crate) fn transform_from(
    canvas: CanvasSize,
    after: usize,
    lift: Option<RegionMask>,
    patch: Patch,
    placement: Placement,
) -> Result<TransformEdit, CoreError> {
    if let Some(lift) = &lift {
        check_mask(canvas, lift)?;
    }
    let [x, y] = patch.origin;
    let [w, h] = patch.size;
    if x.saturating_add(w) > canvas.width() || y.saturating_add(h) > canvas.height() {
        return Err(invalid("patch"));
    }
    Ok(TransformEdit {
        after,
        lift,
        patch,
        placement,
    })
}

impl Artwork {
    /// Transforms the active layer above its strokes so far: clears `lift` (everything when
    /// `None`) and draws `patch`, the pixels lifted from it, through `placement`.
    ///
    /// # Errors
    /// `LayerLocked` when the active layer is locked; `InvalidValue` when the lift or the patch
    /// leaves the canvas.
    pub fn transform_region(
        &self,
        lift: Option<RegionMask>,
        patch: Patch,
        placement: Placement,
    ) -> Result<Applied, CoreError> {
        let index = self.editable_layer(self.active())?;
        let after = self.state().layers[index].strokes.len();
        let edit = transform_from(self.canvas(), after, lift, patch, placement)?;
        let artwork = push_edit(self, index, RegionEdit::Transform(edit));
        Ok(Applied {
            artwork,
            change: Change::Content,
        })
    }
}
