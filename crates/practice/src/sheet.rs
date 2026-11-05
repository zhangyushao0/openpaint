//! The sheet the app lays a session's exercises out on, a pair of frames each (the original at
//! the left, the empty frame for the copy at its right), three pairs a row; a long colour copy's
//! sheet, one pair of frames shaped as its picture (and read back from its canvas, whatever
//! became of the picture); a picture filling its frame.
//!
//! Frames follow their picture (S99, D103): an exercise copied from a picture (a single shape, a
//! division, a whole copy, value masses) has both its frames cropped exactly to the picture, its
//! longer side the frame's length, so one edge measures both; every other kind's frames are
//! square. Each pair keeps its place on the sheet (its square slot), the frames of a picture's
//! shape held together in the middle of it, so the sheet's size and every other pair's frames
//! never depend on the pictures.

use crate::pack::Pack;
use crate::path::Point;
use crate::record::SessionRecord;

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

/// The sheet a session's exercises lie on.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// Width and height in canvas pixels.
    pub size: [u32; 2],
    /// Each exercise's frames, in order: three pairs a row, rows down the sheet.
    pub pairs: Vec<Pair>,
}

impl Sheet {
    /// The sheet for exercises whose frames are each square (`None`) or of a picture of
    /// `[width, height]` pixels: in its slot, the original's frame against the gap from the
    /// left, the copy's from the right, both in the slot's middle from top to bottom.
    #[must_use]
    pub fn of(pictures: &[Option<[u32; 2]>]) -> Self {
        let pair_width = 2.0 * FRAME + PAIR_GAP;
        let rows = pictures.len().div_ceil(COLUMNS).max(1);
        let pairs = (pictures.iter().enumerate())
            .map(|(index, picture)| {
                let (column, row) = ((index % COLUMNS) as f64, (index / COLUMNS) as f64);
                let x = EDGE + column * (pair_width + COLUMN_GAP);
                let y = EDGE + row * (FRAME + ROW_GAP);
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
            })
            .collect();
        let width = 2.0 * EDGE + COLUMNS as f64 * pair_width + (COLUMNS - 1) as f64 * COLUMN_GAP;
        let height = 2.0 * EDGE + rows as f64 * FRAME + (rows - 1) as f64 * ROW_GAP;
        Self {
            size: [width as u32, height as u32],
            pairs,
        }
    }

    /// The sheet a session's or a drill's `record` lies on, its pictures as `packs` hold them:
    /// an exercise of a picture in frames of its shape, one no pack holds now (removed) in
    /// square ones, as every other kind.
    #[must_use]
    pub fn of_session(record: &SessionRecord, packs: &[Pack]) -> Self {
        let pictures: Vec<Option<[u32; 2]>> = (record.exercises.iter())
            .map(|exercise| {
                let item = exercise.picture.as_ref()?;
                packs
                    .iter()
                    .find_map(|pack| pack.find(item))
                    .map(|i| i.size)
            })
            .collect();
        Self::of(&pictures)
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
