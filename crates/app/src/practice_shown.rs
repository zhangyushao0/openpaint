//! What lies over a compared copy on a practice sheet as the screen draws it (`Overlay`): each
//! kind's comparison as the last Compare found the copy, a painted copy's reading among it (a
//! colour combination's `AreasRead`, a key shift's `KeyRead`, S107). Made by the sheet
//! (`practice_overlay.rs`), drawn by the UI.

use std::sync::Arc;

use openpaint_practice::{
    AreasCopy, BoxCheck, DivisionCheck, EllipseCheck, FlatCheck, FreeCheck, KeyCheck, MassesCheck,
    Point, Rgb8, ScaleCheck, SolidCheck, SpaceCheck,
};

use crate::practice_pictures::PictureImage;

/// A colour combination's copy as read at the last Compare, and the edges where it made a weak
/// contrast strong (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct AreasRead {
    pub copy: AreasCopy,
    pub strong: Vec<Vec<Point>>,
}

/// A key shift's copy as read at the last Compare (S107): each area's colour in it (none where not
/// painted), what is off, the outlines of the areas marked and the merged pairs' shared edges
/// (frame units), and both frames in grey for the Grey view (the original's none where its
/// picture was not read).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyRead {
    pub colours: Vec<Option<Rgb8>>,
    pub check: KeyCheck,
    pub outlines: Vec<(usize, Vec<Vec<Point>>)>,
    pub edges: Vec<Vec<Point>>,
    pub grey_copy: Arc<PictureImage>,
    pub grey_original: Option<Arc<PictureImage>>,
}

impl KeyRead {
    /// The outline of `area`, if it is marked.
    #[must_use]
    pub fn outline(&self, area: usize) -> Option<&[Vec<Point>]> {
        (self.outlines.iter()).find_map(|(marked, lines)| (*marked == area).then_some(&lines[..]))
    }
}

/// The comparison over the copy's frame, as the last Compare found the copy.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    /// The original's lines where they lie over the copy (in place or aligned), in canvas pixels.
    Lines(Vec<Vec<[f32; 2]>>),
    /// A single shape's, a colour copy's or a whole copy's picture over the copy, `shown` strong
    /// (a whole copy's division hidden under it, S95), its corners (top
    /// left, top right, bottom right, bottom left) in canvas pixels, with a shape's outline's
    /// lines (none without one), in place or aligned.
    Picture {
        corners: [[f32; 2]; 4],
        outline: Vec<Vec<[f32; 2]>>,
        shown: f32,
    },
    /// The swatch and the first and last colours laid.
    Colour {
        original: Rgb8,
        first: Rgb8,
        last: Rgb8,
    },
    /// A colour combination's picture over the copy in place, `shown` strong; the copy as read
    /// (none until its pixels are read), the area the card compares and its outline (frame
    /// units).
    Areas {
        shown: f32,
        read: Option<Arc<AreasRead>>,
        chosen: usize,
        outline: Arc<Vec<Vec<Point>>>,
    },
    /// A key shift (S107): nothing laid over the copy (it has no answer); the copy as read (none
    /// until its pixels are read), the area the card shows and its outline (frame units), and
    /// whether both frames show in grey.
    KeyShift {
        read: Option<Arc<KeyRead>>,
        chosen: usize,
        outline: Arc<Vec<Vec<Point>>>,
        grey: bool,
    },
    /// A value scale's copy as checked (none until its pixels are read).
    Scale(Option<Arc<ScaleCheck>>),
    /// A completed box: the exact box lies over the copy, the copy's edges extended towards its
    /// vanishing points (frame units).
    Box(Arc<BoxCheck>),
    /// Free boxes: each one's edges extended to the horizon, where each direction meets it and
    /// its front corners under 90 degrees (frame units).
    FreeBoxes(Arc<FreeCheck>),
    /// An ellipse: the exact one and the one fitted to the copy's lines, and their differences.
    Ellipse(Arc<EllipseCheck>),
    /// An estimate on the flat: the last mark or line read (frame units) and its error.
    Flat(Arc<FlatCheck>),
    /// A point in space: the last mark or lines read (frame units) and the error on the face.
    Space(Arc<SpaceCheck>),
    /// A solid in its box: the curves and sides read (frame units), the ellipses fitted.
    Solid(Arc<SolidCheck>),
    /// An object simplified into its box (S94): the curves and edges read against its visible
    /// edges and outlines.
    Object(Arc<SolidCheck>),
    /// A division (S95): its picture in place under the copy, its corners (top left, top
    /// right, bottom right, bottom left) in canvas pixels, `shown` strong; the lines as drawn,
    /// joined, by level (frame units), and each level's count and length.
    Division {
        corners: [[f32; 2]; 4],
        shown: f32,
        check: Arc<DivisionCheck>,
    },
    /// Value masses (S96): the answer's greys (the sheet's painted picture) over the picture's
    /// place in the copy's frame, its corners (top left, top right, bottom right, bottom left) in
    /// canvas pixels, `shown` strong, its mass edges (the sheet picture's) red over it; the copy
    /// as read (none until its pixels arrive): the hatching where it differs.
    Masses {
        corners: [[f32; 2]; 4],
        shown: f32,
        read: Option<Arc<MassesCheck>>,
    },
}
