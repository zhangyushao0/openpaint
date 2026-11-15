//! Practice (G9): lessons of the course's exercises on practice sheets, and long colour copies
//! beside them. A practice sheet is stored like an artwork (its strokes and fills through the
//! same history, autosave and library), but the gallery never lists it: it belongs to its
//! sheet's record, which practice-store keeps in the library's record space. The
//! open sheet's flow is `sheet.rs` (Compare in `compare.rs`, a painted copy's
//! reading and the eyedropper in `read.rs`, its views in `view.rs`), its pens
//! `pens.rs`, the copy read from its records `copy.rs`; the log of every
//! sheet and copy `log.rs`.
//!
//! This module holds what the user asks of a sheet and what the screen draws of it (what lies
//! over a compared copy: `shown.rs`).

use std::sync::Arc;

use openpaint_core::Color;
use openpaint_practice::{
    CardItem, Exercise, First, Given, Kind, Layout, Original, Outcome, Pack, Pair, Point, Rgb8,
    Sheet, SheetMark, SheetPicture,
};

use crate::lesson::{End, Judged, LessonRow, LessonShown};
use crate::pictures::PictureImage;
pub use crate::shown::CopyRead;
use openpaint_app::Availability;
use openpaint_app::LayerRefusal;
use openpaint_app::Title;
use openpaint_app::Viewport;

/// How much of a colour combination's, a colour copy's, a division's or a whole copy's original
/// shows over the copy at first, in place.
pub const SHOWN: f32 = 0.5;

/// What the user can ask of a practice sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticeIntent {
    /// The shown exercise's kind's How, shown (true) or closed.
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
    /// On to the next exercise, the view moved to it within `viewport` at its zoom, whether or not
    /// a result was chosen; after the last the lesson ends and its end shows over the rows used.
    Next(Viewport),
    /// The lesson the end offers (`End::next`), from the end: on the same sheet from a new row
    /// where it fits (the view gliding to it within `viewport`), else on a new one.
    OneMore(Viewport),
    /// The result the user chooses for the shown exercise after Compare, where the eye judges
    /// (S122: `Judged`, `Verdict::Eye`): kept once, with `Judge::Eye`; refused once one is kept.
    Judge(Outcome),
    /// Closes the notice.
    Dismiss,
}

/// The pens an exercise's own tools offer, as its kind's facts list them (`pens.rs`).
pub use openpaint_practice::Pen;

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
    /// The lesson is done.
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
    /// The sheet's frames and what they print; the first `revealed` pairs show.
    pub sheet: Arc<SheetView>,
    pub revealed: usize,
    /// The shown exercise's original, made by its generator (the only one made; none once done).
    pub original: Option<Arc<dyn Original>>,
    /// Whether it is a long colour copy's sheet.
    pub copy: bool,
    /// The exercise shown, an index into the sheet's; none once the lesson is done.
    pub current: Option<usize>,
    /// Whether Next ends the lesson.
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
    /// The pictures the compared copy's reading brought, which its marks lay (`Source::Read`: a
    /// key shift's frames in grey, S107).
    pub read_pictures: Option<Arc<Vec<PictureImage>>>,
    pub notice: Option<PracticeNotice>,
    /// Whether Aligned can be chosen for the shown exercise: lines, or a picture with an outline.
    pub aligned_offered: bool,
    /// Aligned is chosen and its fit is still being found (off the frame loop).
    pub fitting: bool,
    /// The sheet's pictures read so far (none: one that could not be read).
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
    /// The lesson in hand as the bar names it (S122); none on a long colour copy's sheet. Once
    /// the lesson is done, still its own (its last exercise's place).
    pub lesson: Option<LessonShown>,
    /// The shown exercise's kind (none once the lesson is done): what its facts, its words and
    /// its How go by (S122).
    pub kind: Option<Kind>,
    /// The How shown, for a kind: by itself the first time a kind comes up outside a warm-up
    /// (compared on no sheet before), else on asking (`PracticeIntent::Instructions`).
    pub how: Option<Kind>,
    /// While comparing: who judges the shown exercise and the result kept (S122).
    pub judged: Option<Judged>,
    /// Once the lesson is done: its end (S122), shown over the rows it used.
    pub end: Option<End>,
    /// The lesson laid after it (One more, or a Pomodoro's chain) was rolled back, the store not
    /// taking it: the end shows even while a block runs, its One more trying again (S124).
    pub rolled_back: bool,
    /// Each result marked under its pair, by the exercise's index among the sheet's: every
    /// lesson's before the one in hand, and its own once it is done (S122).
    pub marked: Vec<(usize, Outcome)>,
}

/// A practice sheet as the screen draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetView {
    /// Its project's title: a colour copy's name.
    pub name: Title,
    pub size: [u32; 2],
    /// Each exercise's frames, in the order of its lessons, at its slot.
    pub pairs: Vec<Pair>,
    /// Each exercise's pack picture (none: a kind without one, or a picture no pack holds now:
    /// removed).
    pub pictures: Vec<Option<Arc<SheetPicture>>>,
    /// Each exercise's picture the practice crate drew for it, as stored (`Given::painted`: a
    /// generated combination's, an object's, value masses' answer in its greys).
    pub painted: Vec<Option<Arc<PictureImage>>>,
    /// What each exercise gives in its frames before Compare, as its kind drew it (S109), as
    /// stored (`Given::marks`).
    pub givens: Vec<Vec<SheetMark>>,
    /// Each exercise's kind, whose facts the flow and the screen read (S122).
    pub kinds: Vec<Kind>,
    /// Each lesson on it as its row's label names it (S122), by its index among the sheet's
    /// lessons; none on a long colour copy's.
    pub rows: Vec<LessonRow>,
}

impl SheetView {
    /// The sheet of `record` on its canvas of `size` pixels, titled `name`: what each exercise
    /// printed as `printed` holds it (stored when the sheet started, `Givens::laid`; no
    /// generator runs), its pictures as `packs` hold them, each in frames of its shape as it was
    /// laid (S99), so they stay where its strokes lie whatever became of its picture; its lessons'
    /// labels `rows` (`PracticeLog::rows`).
    #[must_use]
    pub(crate) fn of(
        record: &Sheet,
        printed: &[Given],
        packs: &[Pack],
        (size, name, rows): ([u32; 2], &Title, Vec<LessonRow>),
    ) -> Self {
        let sheet = Layout::on(record, size);
        let exercises: Vec<&Exercise> = record.exercises().map(|laid| &laid.exercise).collect();
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
            size: sheet.size,
            pairs: sheet.pairs,
            pictures,
            painted,
            givens,
            kinds: exercises.iter().map(|e| e.kind).collect(),
            rows,
        }
    }

    /// It with `record`'s last lesson, just appended, laid on it as `printed` holds what that
    /// lesson's exercises print, its pictures as `packs` hold them, its lessons' labels `rows`.
    #[must_use]
    pub(crate) fn appended(
        &self,
        record: &Sheet,
        printed: &[Given],
        packs: &[Pack],
        rows: Vec<LessonRow>,
    ) -> Self {
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
            (self.size, &self.name, Vec::new()),
        );
        let mut joined = self.clone();
        joined.pairs = Layout::on(record, self.size).pairs;
        joined.pictures.append(&mut sheet.pictures);
        joined.painted.append(&mut sheet.painted);
        joined.givens.append(&mut sheet.givens);
        joined.kinds.append(&mut sheet.kinds);
        joined.rows = rows;
        joined
    }
}
