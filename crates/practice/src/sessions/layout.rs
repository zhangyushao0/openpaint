//! The sheet the app lays a session's exercises out on, a pair of frames each (the original at
//! the left, the empty frame for the copy at its right), three pairs a row, each exercise at its
//! slot, a lesson appended from the row after the last one used (`next_slot`); a long colour copy's
//! sheet, one pair of frames shaped as its picture (and read back from its canvas, whatever
//! became of the picture); a picture filling its frame.
//!
//! Frames follow their picture (S99, D103): an exercise copied from a picture (a single shape, a
//! division, a whole copy, value masses) has both its frames cropped exactly to the picture, its
//! longer side the frame's length, so one edge measures both; every other kind's frames are
//! square. Each pair keeps its place on the sheet (its square slot), the frames of a picture's
//! shape held together in the middle of it, so the sheet's size and every other pair's frames
//! never depend on the pictures.

use crate::maths::path::Point;
use crate::records::sheet::{Laid, Sheet};

/// A frame's side on the sheet (a picture's frames' longer side), in canvas pixels.
pub const FRAME: f64 = 400.0;
/// A long colour copy's frames' longer side, in canvas pixels: room for 10 to 20 hours of detail.
pub const STUDY_SIDE: f64 = 1600.0;
/// Strokes may begin this share of a frame's longer side outside it (and are clipped to it).
pub const FRAME_REACH: f64 = 0.1;
/// Between the original's frame and the copy's.
const PAIR_GAP: f64 = 40.0;
/// Between pairs, across and down, and round the sheet's edge.
const COLUMN_GAP: f64 = 120.0;
const ROW_GAP: f64 = 120.0;
const EDGE: f64 = 80.0;
/// Pairs a row.
const COLUMNS: usize = 3;

/// Where a picture lies in its frame: filling it, the frame of its own shape (S99).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PictureFit {
    /// Frame units per picture pixel.
    pub scale: f64,
    /// Its width and height, in frame units: the frame's (`Frame::extent`).
    pub size: Point,
}

impl PictureFit {
    /// The fit of a picture of `size` pixels in its frame.
    #[must_use]
    pub fn of(size: [u32; 2]) -> Self {
        let [width, height] = size.map(|side| f64::from(side.max(1)));
        let scale = 1.0 / width.max(height);
        Self {
            scale,
            size: [width * scale, height * scale],
        }
    }

    /// The frame point of the picture's pixel point `p`.
    #[must_use]
    pub fn to_frame(&self, p: Point) -> Point {
        [p[0] * self.scale, p[1] * self.scale]
    }

    /// The picture's pixel point of the frame point `p`.
    #[must_use]
    pub fn to_picture(&self, p: Point) -> Point {
        [p[0] / self.scale, p[1] / self.scale]
    }

    /// Its corners in frame units: top left, top right, bottom right, bottom left.
    #[must_use]
    pub const fn corners(&self) -> [Point; 4] {
        let [w, h] = self.size;
        [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]]
    }
}

/// A frame on the sheet, in canvas pixels: square, or of its picture's shape. Frame units run
/// evenly on both axes, one to the frame's longer side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub origin: Point,
    /// Width and height.
    pub size: Point,
}

impl Frame {
    /// A square frame of `side` pixels at `origin`.
    #[must_use]
    pub const fn square(origin: Point, side: f64) -> Self {
        Self {
            origin,
            size: [side, side],
        }
    }

    /// Canvas pixels per frame unit: the frame's longer side.
    #[must_use]
    pub const fn unit(&self) -> f64 {
        self.size[0].max(self.size[1])
    }

    /// Its width and height in frame units: 1 along its longer side.
    #[must_use]
    pub fn extent(&self) -> Point {
        [self.size[0] / self.unit(), self.size[1] / self.unit()]
    }

    /// Its middle, in canvas pixels.
    #[must_use]
    pub fn middle(&self) -> Point {
        [
            self.origin[0] + self.size[0] / 2.0,
            self.origin[1] + self.size[1] / 2.0,
        ]
    }

    /// The canvas point of the frame point `p` (frame units).
    #[must_use]
    pub fn to_canvas(&self, p: Point) -> Point {
        [
            self.origin[0] + p[0] * self.unit(),
            self.origin[1] + p[1] * self.unit(),
        ]
    }

    /// The frame point (frame units) of the canvas point `p`.
    #[must_use]
    pub fn to_frame(&self, p: Point) -> Point {
        [
            (p[0] - self.origin[0]) / self.unit(),
            (p[1] - self.origin[1]) / self.unit(),
        ]
    }

    /// Whether the canvas point `p` lies in the frame grown by `margin` pixels on every side.
    #[must_use]
    pub fn contains(&self, p: Point, margin: f64) -> bool {
        (0..2).all(|axis| {
            p[axis] >= self.origin[axis] - margin
                && p[axis] <= self.origin[axis] + self.size[axis] + margin
        })
    }

    /// Whether a stroke begun at the canvas point `p` belongs to the frame: in it or within
    /// `FRAME_REACH` of it.
    #[must_use]
    pub fn holds(&self, p: Point) -> bool {
        self.contains(p, FRAME_REACH * self.unit())
    }

    /// Its corners, top left first, clockwise, in canvas pixels.
    #[must_use]
    pub fn corners(&self) -> [Point; 4] {
        let ([x, y], [w, h]) = (self.origin, self.size);
        [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
    }
}

/// One exercise's frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    pub original: Frame,
    pub copy: Frame,
}

impl Pair {
    /// The middle of both frames, in canvas pixels.
    #[must_use]
    pub fn centre(&self) -> Point {
        [
            (self.original.origin[0] + self.copy.origin[0] + self.copy.size[0]) / 2.0,
            self.original.origin[1] + self.original.size[1] / 2.0,
        ]
    }
}

/// How a session's exercises lie on its sheet: the sheet's size and each exercise's frames.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Width and height in canvas pixels.
    pub size: [u32; 2],
    /// Each exercise's frames, in order: three pairs a row, rows down the sheet.
    pub pairs: Vec<Pair>,
}

/// A pair's width: both frames and the gap between.
const PAIR_WIDTH: f64 = 2.0 * FRAME + PAIR_GAP;
/// A day's sheet's width, and a row's height with its gap below.
const WIDTH: f64 = 2.0 * EDGE + COLUMNS as f64 * PAIR_WIDTH + (COLUMNS - 1) as f64 * COLUMN_GAP;
const ROW: f64 = FRAME + ROW_GAP;

impl Layout {
    /// The sheet for exercises whose frames are each square (`None`) or of a picture of
    /// `[width, height]` pixels, each in the slot of its index (`pair_at`).
    #[must_use]
    pub fn of(pictures: &[Option<[u32; 2]>]) -> Self {
        let rows = pictures.len().div_ceil(COLUMNS).max(1);
        let pairs = (pictures.iter().enumerate())
            .map(|(slot, picture)| Self::pair_at(slot, *picture))
            .collect();
        Self {
            size: Self::of_rows(rows),
            pairs,
        }
    }

    /// The frames of slot `slot` for a picture of `picture` pixels (`None`: square): the
    /// original's frame against the gap from the left, the copy's from the right, both in the
    /// slot's middle from top to bottom.
    #[must_use]
    pub fn pair_at(slot: usize, picture: Option<[u32; 2]>) -> Pair {
        let (column, row) = ((slot % COLUMNS) as f64, (slot / COLUMNS) as f64);
        let x = EDGE + column * (PAIR_WIDTH + COLUMN_GAP);
        let y = EDGE + row * ROW;
        let size = picture.map_or([FRAME; 2], |size| {
            PictureFit::of(size).size.map(|side| side * FRAME)
        });
        let top = y + (FRAME - size[1]) / 2.0;
        Pair {
            original: Frame {
                origin: [x + FRAME - size[0], top],
                size,
            },
            copy: Frame {
                origin: [x + FRAME + PAIR_GAP, top],
                size,
            },
        }
    }

    /// A day's sheet of `rows` rows.
    fn of_rows(rows: usize) -> [u32; 2] {
        let height = 2.0 * EDGE + rows as f64 * FRAME + rows.saturating_sub(1) as f64 * ROW_GAP;
        [WIDTH as u32, height as u32]
    }

    /// The layout `sheet` lies on, from its exercises as laid: a long colour copy's of its
    /// picture's shape (`study`), else one pair an exercise at its slot, a picture's frames of
    /// its shape, every other kind's square, on a sheet of the rows they use (`used`).
    #[must_use]
    pub fn of_sheet(sheet: &Sheet) -> Self {
        let frame = |laid: &Laid| (laid.frame != [0, 0]).then(|| laid.frame.map(u32::from));
        if sheet.is_copy() {
            let picture = sheet.exercises().next().and_then(frame);
            return Self::study(picture.unwrap_or([1, 1]));
        }
        let pairs = (sheet.exercises())
            .map(|laid| Self::pair_at(usize::from(laid.slot), frame(laid)))
            .collect();
        Self {
            size: Self::used(sheet),
            pairs,
        }
    }

    /// `sheet`'s layout on its canvas of `canvas` pixels: the same frames.
    #[must_use]
    pub fn on(sheet: &Sheet, canvas: [u32; 2]) -> Self {
        Self {
            size: canvas,
            ..Self::of_sheet(sheet)
        }
    }

    /// The size of the rows `sheet`'s exercises use (one at least); a colour copy's whole sheet.
    #[must_use]
    pub fn used(sheet: &Sheet) -> [u32; 2] {
        if sheet.is_copy() {
            return Self::of_sheet(sheet).size;
        }
        Self::of_rows((Self::next_slot(sheet) / COLUMNS).max(1))
    }

    /// The first slot of the row after the last one `sheet`'s exercises use: where a lesson
    /// appended to it starts (0 on an empty sheet).
    #[must_use]
    pub fn next_slot(sheet: &Sheet) -> usize {
        let rows = (sheet.exercises())
            .map(|laid| usize::from(laid.slot) / COLUMNS + 1)
            .max()
            .unwrap_or(0);
        rows * COLUMNS
    }

    /// How many rows a canvas of `size` pixels holds.
    #[must_use]
    pub fn rows_on(size: [u32; 2]) -> usize {
        let height = f64::from(size[1]) - 2.0 * EDGE + ROW_GAP;
        (height / ROW).floor().max(0.0) as usize
    }

    /// Whether a lesson of `exercises` fits on `sheet`'s canvas of `size` pixels from the row
    /// after its last one used; never on a colour copy's.
    #[must_use]
    pub fn fits(sheet: &Sheet, exercises: usize, size: [u32; 2]) -> bool {
        let rows = Self::next_slot(sheet) / COLUMNS + exercises.div_ceil(COLUMNS);
        !sheet.is_copy() && rows <= Self::rows_on(size)
    }

    /// The rectangle of slot `slot`'s row, `[x, y, width, height]` in canvas pixels: the row with
    /// the edge round it, exactly a one-row sheet.
    #[must_use]
    pub fn row_rect(slot: usize) -> [f64; 4] {
        let [width, height] = Self::of_rows(1).map(f64::from);
        [0.0, (slot / COLUMNS) as f64 * ROW, width, height]
    }

    /// A long colour copy's sheet for a picture of `picture` pixels: one pair of frames of the
    /// picture's shape, their longer side `STUDY_SIDE`.
    #[must_use]
    pub fn study(picture: [u32; 2]) -> Self {
        let [width, height] = picture.map(|side| f64::from(side.max(1)));
        let scale = STUDY_SIDE / width.max(height);
        let size = [(width * scale).round(), (height * scale).round()];
        let original = Frame {
            origin: [EDGE, EDGE],
            size,
        };
        let copy = Frame {
            origin: [EDGE + size[0] + PAIR_GAP, EDGE],
            size,
        };
        let sheet = [2.0 * EDGE + 2.0 * size[0] + PAIR_GAP, 2.0 * EDGE + size[1]];
        Self {
            size: sheet.map(|side| side as u32),
            pairs: vec![Pair { original, copy }],
        }
    }

    /// The long colour copy's sheet `study` laid on a canvas of `size` pixels, read back from the
    /// canvas: the same frames whatever became of the picture.
    #[must_use]
    pub fn study_on(size: [u32; 2]) -> Self {
        let [width, height] = size.map(f64::from);
        let frame = [(width - 2.0 * EDGE - PAIR_GAP) / 2.0, height - 2.0 * EDGE];
        Self::study(frame.map(|side| side.round().max(1.0) as u32))
    }
}
