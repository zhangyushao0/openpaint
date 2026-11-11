//! Practice (G9): sessions of the course's exercises on practice sheets, and long colour copies
//! beside them. A practice sheet is stored like an artwork (its strokes and fills through the
//! same history, autosave and library), but the gallery never lists it: it belongs to its
//! session's record, which practice-store keeps in the library's record space. The
//! open sheet's flow is `sheet.rs` (Compare in `compare.rs`, a painted copy's
//! reading and the eyedropper in `read.rs`, its views in `view.rs`), its pens
//! `pens.rs`, the copy read from its records `copy.rs`; the log of every
//! session and copy `log.rs`.
//!
//! This module holds what the user asks of a sheet and what the screen draws of it (what lies
//! over a compared copy: `shown.rs`).

use std::sync::Arc;

use openpaint_core::Color;
use openpaint_practice::{
    CardItem, Exercise, Family, First, Given, KindSummary, Layout, Level, Original, Pack, Pair,
    Point, Rgb8, Sheet, SheetMark, SheetPicture, Value,
};

use crate::pictures::PictureImage;
use crate::sheet::Finished;
pub use crate::shown::{CopyRead, Greys};
use openpaint_app::Availability;
use openpaint_app::LayerRefusal;
use openpaint_app::Title;
use openpaint_app::Viewport;

/// How much of a colour combination's, a colour copy's, a division's or a whole copy's original
/// shows over the copy at first, in place.
pub const SHOWN: f32 = 0.5;

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
    /// The way to see a compared geometric answer laid faint over it (true) or taken away (S97,
    /// 看画法; S98 every kind checked as geometry); each Compare starts without it.
    Construction(bool),
    /// How much of a colour combination's, a colour copy's, a division's or a whole copy's
    /// original shows over the copy, 0 to 1.
    Shown(f32),
    /// The colour combination's area the card compares on the wheel (a key shift's whose
    /// lightness it shows).
    Area(usize),
    /// A compared key shift's frames in grey (true) or in colour (S107).
    Grey(bool),
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
    /// Another session (after a drill, another drill of its kind), from the summary: on the same
    /// sheet from a new row where it fits (the view gliding to it within `viewport`), else on a
    /// new one.
    OneMore(Viewport),
    /// Closes the notice.
    Dismiss,
}

/// The pens an exercise's own tools offer (`pens.rs`).
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
    /// A division's levels in V大's colours (S95): its main lines red, the second green, the
    /// third blue.
    Main,
    Second,
    Third,
    /// Value masses' three greys (S96): the dark, the middle and the light.
    Dark,
    Middle,
    Light,
}

impl Pen {
    /// The pens a line or a shape is drawn with, and those a colour combination or a value scale
    /// is painted with.
    pub const DRAWING: [Self; 3] = [Self::Sketch, Self::Draw, Self::Correct];
    pub const PAINTING: [Self; 3] = [Self::Big, Self::Small, Self::Soft];
    /// A division's pens, and a whole copy's: its division's, then Draw for its line drawing (and
    /// the painting pens for its colour blocks, S96).
    pub const DIVIDING: [Self; 3] = [Self::Main, Self::Second, Self::Third];
    pub const COPYING: [Self; 4] = [Self::Main, Self::Second, Self::Third, Self::Draw];
    /// Value masses' greys (S96).
    pub const VALUES: [Self; 3] = [Self::Dark, Self::Middle, Self::Light];

    /// Whether it paints in the colour picked.
    #[must_use]
    pub fn paints(self) -> bool {
        Self::PAINTING.contains(&self)
    }

    /// The value it paints, if it is one of value masses' greys.
    #[must_use]
    pub const fn value(self) -> Option<Value> {
        match self {
            Self::Dark => Some(Value::Dark),
            Self::Middle => Some(Value::Middle),
            Self::Light => Some(Value::Light),
            _ => None,
        }
    }

    /// The division's level it draws, if it is a division's pen.
    #[must_use]
    pub const fn level(self) -> Option<Level> {
        match self {
            Self::Main => Some(Level::Main),
            Self::Second => Some(Level::Second),
            Self::Third => Some(Level::Third),
            _ => None,
        }
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
    /// The eyedropper over a key shift's original, which is never picked (S107).
    OriginalLocked,
    /// The frame could not be filled: the layer is hidden or locked.
    CannotFill(LayerRefusal),
    /// The session is done.
    Done,
    /// The lesson One more appended could not be stored: it was rolled back.
    NotSaved,
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
    /// The session's frames and what they print; the first `revealed` pairs show.
    pub sheet: Arc<SheetView>,
    pub revealed: usize,
    /// The shown exercise's original, made by its generator (the only one made; none once done).
    pub original: Option<Arc<Original>>,
    /// A session's stage; none for a drill or a long colour copy.
    pub stage: Option<u8>,
    /// Whether it is a long colour copy's sheet.
    pub copy: bool,
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
    /// A compared geometric answer's construction shows (S97, S98).
    pub construction: bool,
    /// The full interface shows over the sheet, else the exercise's own tools.
    pub all_tools: bool,
    /// The pen the exercise's own tools draw with now and its diameter (canvas pixels); `None`
    /// while they do not show or the toolbox's eraser is used.
    pub pen: Option<(Pen, f32)>,
    /// The shown exercise's first Compare as stored, once compared.
    pub result: Option<First>,
    /// What lies over the copy while comparing.
    pub overlay: Option<Vec<SheetMark>>,
    /// What the card shows of the comparison under its title (S109).
    pub card: Vec<CardItem>,
    /// A compared key shift's frames in grey, while the Grey view shows them (S107).
    pub greys: Option<Arc<Greys>>,
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
    /// Each single shape's picture read with its ground faded, as its original's frame shows it
    /// (S99); Compare lays the picture itself.
    pub grounded: Vec<(openpaint_practice::ItemRef, Arc<PictureImage>)>,
    /// A value scale's cell the pen paints and Fill fills.
    pub cell: Option<usize>,
    /// The last point the eyedropper compared, while comparing.
    pub probe: Option<Probe>,
    /// The comparison shown, as its kind found the copy (S109): what `overlay` and `card` come
    /// from.
    pub compared: Option<openpaint_practice::Comparison>,
    /// How strongly the original shows over the copy, the area the card compares and whether a
    /// key shift's frames show in grey, as the user set them.
    pub shown: f32,
    pub area: usize,
    pub grey: bool,
}

/// A session's sheet as the screen draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetView {
    /// Its project's title: a colour copy's name.
    pub name: Title,
    /// The number of its lesson in hand among the lessons of its kind: a session's in its stage,
    /// a drill's among its kind's drills, a copy's among the copies (`openpaint_practice::number`).
    pub number: u32,
    pub size: [u32; 2],
    /// Each exercise's frames, in the order of its lessons, at its slot.
    pub pairs: Vec<Pair>,
    /// Each exercise's family, whose facts the flow and the screen read.
    pub families: Vec<Family>,
    /// Each exercise's pack picture (none: a kind without one, or a picture no pack holds now:
    /// removed).
    pub pictures: Vec<Option<Arc<SheetPicture>>>,
    /// Each exercise's picture the practice crate drew for it, as stored (`Given::painted`: a
    /// generated combination's, an object's, value masses' answer in its greys).
    pub painted: Vec<Option<Arc<PictureImage>>>,
    /// What each exercise gives in its frames before Compare, as its kind drew it (S109), as
    /// stored (`Given::marks`).
    pub givens: Vec<Vec<SheetMark>>,
}

impl SheetView {
    /// The sheet of `record` on its canvas of `size` pixels, titled `name`: what each exercise
    /// printed as `printed` holds it (stored when the sheet started, `Givens::laid`; no
    /// generator runs), its pictures as `packs` hold them, each in frames of its shape as it was
    /// laid (S99), so they stay where its strokes lie whatever became of its picture.
    #[must_use]
    pub fn of(
        record: &Sheet,
        printed: &[Given],
        packs: &[Pack],
        (size, name, number): ([u32; 2], &Title, u32),
    ) -> Self {
        let sheet = Layout::on(record, size);
        let exercises: Vec<&Exercise> = record.exercises().map(|laid| &laid.exercise).collect();
        let families: Vec<Family> = exercises.iter().map(|e| e.kind.family()).collect();
        let pictures: Vec<Option<Arc<SheetPicture>>> = (exercises.iter())
            .map(|e| SheetPicture::of(e.picture.as_ref()?, packs).map(Arc::new))
            .collect();
        let painted = (printed.iter())
            .map(|given| {
                let (size, pixels) = given.painted.clone()?;
                Some(Arc::new(PictureImage { size, pixels }))
            })
            .collect();
        let givens = printed.iter().map(|given| given.marks.clone()).collect();
        Self {
            name: name.clone(),
            number,
            size: sheet.size,
            pairs: sheet.pairs,
            pictures,
            painted,
            families,
            givens,
        }
    }

    /// It with `record`'s last lesson, just appended, laid on it as `printed` holds what that
    /// lesson's exercises print, its pictures as `packs` hold them, numbered `number`.
    #[must_use]
    pub fn appended(&self, record: &Sheet, printed: &[Given], packs: &[Pack], number: u32) -> Self {
        let Some(lesson) = record.lessons.last() else {
            return self.clone();
        };
        let mut sheet = Self::of(
            &Sheet {
                lessons: vec![lesson.clone()],
                ..record.clone()
            },
            printed,
            packs,
            (self.size, &self.name, number),
        );
        let mut joined = self.clone();
        joined.number = number;
        joined.pairs = Layout::on(record, self.size).pairs;
        joined.families.append(&mut sheet.families);
        joined.pictures.append(&mut sheet.pictures);
        joined.painted.append(&mut sheet.painted);
        joined.givens.append(&mut sheet.givens);
        joined
    }
}
