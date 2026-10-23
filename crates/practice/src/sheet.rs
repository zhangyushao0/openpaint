//! The sheet the app lays a session's exercises out on, a pair of frames each (the original at
//! the left, the empty frame for the copy at its right), three pairs a row; a long colour copy's
//! sheet, one pair of frames shaped as its picture (and read back from its canvas, whatever
//! became of the picture); a picture fitted inside its frame.

use crate::path::Point;

/// The margin a picture keeps inside its frame, a share of the frame's side.
pub const PICTURE_MARGIN: f64 = 0.05;
/// A frame's side on the sheet, in canvas pixels.
pub const FRAME: f64 = 400.0;
/// A long colour copy's frames' longer side, in canvas pixels: room for 10 to 20 hours of detail.
pub const STUDY_SIDE: f64 = 1600.0;
/// Between the original's frame and the copy's.
const PAIR_GAP: f64 = 40.0;
/// Between pairs, across and down, and round the sheet's edge.
const COLUMN_GAP: f64 = 120.0;
const ROW_GAP: f64 = 120.0;
const EDGE: f64 = 80.0;
/// Pairs a row.
const COLUMNS: usize = 3;

/// Where a picture lies in its frame: evenly scaled to fit inside `PICTURE_MARGIN`, centred, or
/// filling a frame of its own shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PictureFit {
    /// Frame units per picture pixel.
    pub scale: f64,
    /// The picture's top left corner, in frame units.
    pub origin: Point,
    /// Its width and height, in frame units.
    pub size: Point,
}

impl PictureFit {
    /// The fit of a picture of `size` pixels in a square frame.
    #[must_use]
    pub fn of(size: [u32; 2]) -> Self {
        let [width, height] = size.map(|side| f64::from(side.max(1)));
        let room = 1.0 - 2.0 * PICTURE_MARGIN;
        let scale = room / width.max(height);
        let shown = [width * scale, height * scale];
        Self {
            scale,
            origin: [(1.0 - shown[0]) / 2.0, (1.0 - shown[1]) / 2.0],
            size: shown,
        }
    }

    /// The fit of a picture of `size` pixels filling a frame of its own shape (a colour copy's).
    #[must_use]
    pub fn filling(size: [u32; 2]) -> Self {
        let [width, height] = size.map(|side| f64::from(side.max(1)));
        Self {
            scale: 1.0 / width,
            origin: [0.0, 0.0],
            size: [1.0, height / width],
        }
    }

    /// The frame point of the picture's pixel point `p`.
    #[must_use]
    pub fn to_frame(&self, p: Point) -> Point {
        [
            self.origin[0] + p[0] * self.scale,
            self.origin[1] + p[1] * self.scale,
        ]
    }

    /// The picture's pixel point of the frame point `p`.
    #[must_use]
    pub fn to_picture(&self, p: Point) -> Point {
        [
            (p[0] - self.origin[0]) / self.scale,
            (p[1] - self.origin[1]) / self.scale,
        ]
    }
}

/// A frame on the sheet, in canvas pixels: square, or a colour copy's of its picture's shape.
/// Frame units run evenly on both axes, one to the frame's width.
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

    /// Canvas pixels per frame unit: the frame's width.
    #[must_use]
    pub const fn unit(&self) -> f64 {
        self.size[0]
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
    /// The sheet for `count` exercises.
    #[must_use]
    pub fn of(count: usize) -> Self {
        let pair_width = 2.0 * FRAME + PAIR_GAP;
        let rows = count.div_ceil(COLUMNS).max(1);
        let pairs = (0..count)
            .map(|index| {
                let (column, row) = ((index % COLUMNS) as f64, (index / COLUMNS) as f64);
                let x = EDGE + column * (pair_width + COLUMN_GAP);
                let y = EDGE + row * (FRAME + ROW_GAP);
                Pair {
                    original: Frame::square([x, y], FRAME),
                    copy: Frame::square([x + FRAME + PAIR_GAP, y], FRAME),
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
