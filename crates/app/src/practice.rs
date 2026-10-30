//! Practice (G9): sessions of the course's exercises on practice sheets, and long colour copies
//! beside them. A practice sheet is stored like an artwork (its strokes and fills through the
//! same history, autosave and library), but the gallery never lists it: it belongs to its
//! session's record, which the store keeps beside the artworks (`ProjectStore::practice`). The
//! open sheet's flow is `practice_sheet.rs` (Compare in `practice_compare.rs`, a painted copy's
//! reading and the eyedropper in `practice_read.rs`, its views in `practice_view.rs`), its pens
//! `practice_pens.rs`, the copy read from its records `practice_copy.rs`; the app's log of every
//! session and copy `practice_log.rs`.
//!
//! This module holds what the user asks of a sheet and what the screen draws of it.

use std::sync::Arc;

use openpaint_core::Color;
use openpaint_practice::{
    AreasCopy, BoxCheck, EllipseCheck, ExerciseResult, Family, FlatCheck, FreeCheck, KindSummary,
    Original, Pack, Pair, Point, Rgb8, ScaleCheck, SessionRecord, Sheet, SolidCheck, SpaceCheck,
};

use crate::layer_rules::LayerRefusal;
use crate::practice_pictures::{PictureImage, SheetPicture};
use crate::practice_sheet::Finished;
use crate::projection::Availability;
use crate::title::Title;
use crate::view::Viewport;

/// The side a colour combination's picture is drawn at, in pixels: one to a frame's canvas pixel.
pub const AREAS_SIDE: u32 = 400;
/// The side an object's picture is drawn at (S94), likewise.
pub const OBJECT_SIDE: u32 = 400;
/// How much of a colour combination's or a colour copy's original shows over the copy at first,
/// in place; and how much of a single shape's picture shows under its outline.
pub const SHOWN: f32 = 0.5;
pub const SHAPE_SHOWN: f32 = 0.35;

/// What the user can ask of a practice session.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticeIntent {
    /// The instructions for the shown exercise's kind, shown (true) or closed.
    Instructions(bool),
    /// The original laid over the copy (true; the first time stores the exercise's result, every
    /// time counts), or taken away.
    Compare(bool),
    /// A line's original laid over the copy aligned onto it (true) or in place.
    Aligned(bool),
    /// How much of a colour combination's or a colour copy's original shows over the copy, 0 to
    /// 1.
    Shown(f32),
    /// The colour combination's area the card compares on the wheel.
    Area(usize),
    /// The pen the exercise's own tools draw or paint with.
    Pen(Pen),
    /// The full interface over the sheet (true), or the exercise's own tools.
    AllTools(bool),
    /// The shown colour's, colour combination's or value scale's cell's frame filled with `Color`
    /// (the wheel beside it).
    Fill(Color),
    /// On to the next exercise, the view moved to it within `viewport` at its zoom; after the
    /// last the session ends and its summary shows over the whole sheet.
    Next(Viewport),
    /// Another session (after a drill, another drill of its kind), from the summary.
    OneMore,
    /// Closes the notice.
    Dismiss,
}

/// The pens an exercise's own tools offer (`practice_pens.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pen {
    /// A light pencil: marks and construction lines, left out of the aligned fit.
    Sketch,
    /// The ink pen the copy is drawn with.
    Draw,
    /// The blue pen that corrects a compared copy.
    Correct,
    /// Painting in the colour picked: a big brush for the masses, a small one for the edges and
    /// the hierarchy, a soft one for soft edges.
    Big,
    Small,
    Soft,
}

impl Pen {
    /// The pens a line or a shape is drawn with, and those a colour combination or a value scale
    /// is painted with.
    pub const DRAWING: [Self; 3] = [Self::Sketch, Self::Draw, Self::Correct];
    pub const PAINTING: [Self; 3] = [Self::Big, Self::Small, Self::Soft];

    /// Whether it paints in the colour picked.
    #[must_use]
    pub fn paints(self) -> bool {
        Self::PAINTING.contains(&self)
    }
}

/// Why the practice sheet did not take a contact or a fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeNotice {
    /// The pen landed outside the shown exercise's empty frame.
    DrawInFrame,
    /// The pen landed in a value scale's frame outside the cells between its ends.
    DrawInCell,
    /// The eyedropper over an original not compared yet.
    PickAfterCompare,
    /// The frame could not be filled: the layer is hidden or locked.
    CannotFill(LayerRefusal),
    /// The session is done.
    Done,
}

/// A point the eyedropper compared on a colour copy or a colour combination: where (frame units),
/// the original's colour there and the copy's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Probe {
    pub at: Point,
    pub original: Rgb8,
    pub copy: Rgb8,
}

/// The open practice sheet for one UI frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeProjection {
    /// The session's frames and originals; the first `revealed` pairs show.
    pub sheet: Arc<SheetView>,
    pub revealed: usize,
    /// The stage (`openpaint_practice::PROJECT` for a long colour copy).
    pub stage: u8,
    /// The session's number in its stage (a drill's among its kind's drills).
    pub number: u32,
    /// The kind a drill drills (S91); none on a session's or a copy's sheet.
    pub drill: Option<Family>,
    /// The exercise shown (none once the session is done), its family (none once done), whether
    /// it is part of the warm-up, its place among the warm-up's exercises or those of its family
    /// after it and how many those are.
    pub current: Option<usize>,
    pub family: Option<Family>,
    pub warming_up: bool,
    pub place: (usize, usize),
    /// Whether Next ends the session.
    pub last: bool,
    pub compare: Availability,
    pub comparing: bool,
    /// A line's original lies over the copy aligned onto it, else in place.
    pub aligned: bool,
    /// The full interface shows over the sheet, else the exercise's own tools.
    pub all_tools: bool,
    /// The pen the exercise's own tools draw with now and its diameter (canvas pixels); `None`
    /// while they do not show or the toolbox's eraser is used.
    pub pen: Option<(Pen, f32)>,
    /// The shown exercise's stored result, once compared.
    pub result: Option<ExerciseResult>,
    /// What lies over the copy while comparing.
    pub overlay: Option<Overlay>,
    /// The instructions shown, for a family.
    pub instructions: Option<Family>,
    /// Once done: this session's means beside the earlier ones', per kind.
    pub summary: Option<Vec<KindSummary>>,
    /// Once a session is done: its dot on its stage's road and the seals it earned (S89).
    pub finished: Option<Finished>,
    pub notice: Option<PracticeNotice>,
    /// Whether Aligned can be chosen for the shown exercise: lines, or a picture with an outline.
    pub aligned_offered: bool,
    /// Aligned is chosen and its fit is still being found (off the frame loop).
    pub fitting: bool,
    /// The session's pictures read so far (none: one that could not be read).
    pub images: Vec<(openpaint_practice::ItemRef, Option<Arc<PictureImage>>)>,
    /// A value scale's cell the pen paints and Fill fills.
    pub cell: Option<usize>,
    /// The last point the eyedropper compared, while comparing.
    pub probe: Option<Probe>,
}

/// A session's sheet as the screen draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetView {
    /// Its project's title: a colour copy's name.
    pub name: Title,
    pub size: [u32; 2],
    pub pairs: Vec<Pair>,
    pub originals: Vec<Original>,
    /// Each exercise's picture, a single shape's or a colour copy's (none: another kind, or a
    /// picture no pack holds now: removed).
    pub pictures: Vec<Option<Arc<SheetPicture>>>,
    /// Each colour combination's picture, `AREAS_SIDE` square, and each object's (S94),
    /// `OBJECT_SIDE` square: pictures the practice crate draws.
    pub painted: Vec<Option<Arc<PictureImage>>>,
}

impl SheetView {
    /// The sheet of `record` on its canvas of `size` pixels, titled `name`, its pictures as
    /// `packs` hold them. A colour copy's frames are read from its canvas, so they stay where
    /// its strokes lie whatever became of its picture.
    #[must_use]
    pub fn of(record: &SessionRecord, packs: &[Pack], (size, name): ([u32; 2], &Title)) -> Self {
        let sheet = if record.is_project() {
            Sheet::study_on(size)
        } else {
            Sheet::of(record.exercises.len())
        };
        let originals: Vec<Original> = record.exercises.iter().map(|e| e.original()).collect();
        let filling = record.is_project();
        Self {
            name: name.clone(),
            size: sheet.size,
            pairs: sheet.pairs,
            pictures: (record.exercises.iter())
                .map(|e| SheetPicture::of(e.picture.as_ref()?, packs, filling).map(Arc::new))
                .collect(),
            painted: (originals.iter())
                .map(|original| match original {
                    Original::Areas(areas) => Some(Arc::new(PictureImage {
                        size: [AREAS_SIDE; 2],
                        pixels: areas.picture(AREAS_SIDE),
                    })),
                    Original::Object(task) => Some(Arc::new(PictureImage {
                        size: [OBJECT_SIDE; 2],
                        pixels: task.picture(OBJECT_SIDE),
                    })),
                    _ => None,
                })
                .collect(),
            originals,
        }
    }
}

/// The sheet a new `record`'s exercises are laid on: a long colour copy's of its picture's shape
/// (as `packs` hold it), a session's three pairs a row.
#[must_use]
pub fn sheet_of(record: &SessionRecord, packs: &[Pack]) -> Sheet {
    if !record.is_project() {
        return Sheet::of(record.exercises.len());
    }
    let picture = (record.exercises.first())
        .and_then(|e| e.picture.as_ref())
        .and_then(|item| packs.iter().find_map(|pack| pack.find(item)));
    Sheet::study(picture.map_or([1, 1], |item| item.size))
}

/// A colour combination's copy as read at the last Compare, and the edges where it made a weak
/// contrast strong (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct AreasRead {
    pub copy: AreasCopy,
    pub strong: Vec<Vec<Point>>,
}

/// The comparison over the copy's frame, as the last Compare found the copy.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    /// The original's lines where they lie over the copy (in place or aligned), in canvas pixels.
    Lines(Vec<Vec<[f32; 2]>>),
    /// A single shape's or a colour copy's picture over the copy, `shown` strong, its corners (top
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
}
