//! The open practice sheet's flow: which exercise shows, where the pen may draw (strokes and
//! fills only in its empty frame, or a value scale's cell, clipped to it), the exercise's own
//! tools or the full interface, the pen drawn exercises are drawn with (a single line, combined
//! lines, a single shape) or painted ones painted with (a colour combination, a value scale;
//! `pens.rs`), Compare and its overlay (`compare.rs`, `overlay.rs`), a painted copy's reading and
//! the eyedropper (`read.rs`, a reading's frames in grey on asking), Next (the view glides to the
//! next pair at its zoom, its row fitted, `view.rs`; after the last the lesson ends over the rows
//! used), One more in place (`append.rs`: the next lesson laid from a new row of the day's
//! sheet), a kind's How (by itself the first time a kind comes up, a warm-up's never), the
//! result chosen by eye and the lesson's end (`lesson.rs`), the time (`time.rs`) and the
//! notices; what the screen draws (`projected.rs`); the lesson's pictures as the store reads
//! them. A long colour copy is a sheet of one pair, the full interface always showing (painted
//! with the toolbox's own brushes, S104) and no Next. The record changes here and goes to the
//! store whenever it did (`save_request`).

use std::collections::HashSet;
use std::sync::Arc;

use openpaint_core::{Artwork, CanvasSize, RegionMask};
use openpaint_practice::{
    Facts, Frame, ItemRef, Kind, Learner, Original, Pair, Pictures, Sheet, Tally, Tools, seconds,
};

use crate::lesson::End;
use crate::pens::PracticePens;
use crate::pictures::{FitJob, PictureImage, grounded};
use crate::projection::{Pen, PracticeIntent, PracticeNotice, SheetView};
use crate::refused::Refused;
use openpaint_app::Availability;
use openpaint_app::CanvasView;

/// A practice sheet opening, as `PracticeLog::practising` makes it (the one place one is made)
/// and `open_sheet` takes it: its record, the sheet as the screen draws it, what its lesson in
/// hand is set beside, and the kinds whose How was told (compared on any sheet, by their facts'
/// keys). The flow works on the lesson in hand.
#[derive(Debug, Clone, PartialEq)]
pub struct Practising {
    pub(crate) record: Sheet,
    pub(crate) view: SheetView,
    pub(crate) beside: Beside,
    pub(crate) told: HashSet<&'static str>,
}

/// What the lesson in hand is set beside, from the log: what every other lesson adds up to (the
/// seals its end stamps), the learner they tell of (its skill's dots before and after it) and the
/// pictures kept (the lesson its end offers next).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Beside {
    pub(crate) others: Tally,
    pub(crate) learner: Learner,
    pub(crate) pictures: Pictures,
}

mod append;
mod block;
mod checks;
mod compare;
mod lesson;
mod overlay;
mod pens;
mod pick;
mod projected;
mod read;
mod time;
mod view;

pub(crate) use pens::{CopyLayer, Layering};
pub use pick::PracticePick;
pub use read::ReadJob;
pub use view::PRACTICE_ROOM;

use compare::Compared;
use read::Painting;

#[derive(Debug)]
pub struct PracticeSheet {
    record: Sheet,
    /// What its lesson in hand is set beside, and that lesson's end once it is done (`lesson.rs`).
    beside: Beside,
    end: Option<End>,
    sheet: Arc<SheetView>,
    /// The shown exercise and its original, the only one its generator makes (`compare.rs`).
    in_hand: Option<(usize, Arc<Original>)>,
    canvas: CanvasSize,
    pens: PracticePens,
    /// What strokes and fills stay inside: the shown exercise's empty frame, or a value scale's
    /// cell.
    clip: Option<RegionMask>,
    /// While comparing: the check as the last Compare found the copy.
    compared: Option<Compared>,
    /// A line's original lies over the copy aligned onto it, else in place.
    aligned: bool,
    /// A compared geometric answer's construction shows (S97, S98); each Compare starts without it.
    construction: bool,
    /// The full interface shows, else the exercise's own tools; a long colour copy's always
    /// (S104: painted with the toolbox's own brushes).
    all_tools: bool,
    /// The pen the exercise draws or paints with now, the last of Sketch and Draw chosen, the
    /// last painting pen chosen, the last division's pen chosen (S95) and value masses' last
    /// grey chosen (S96).
    pen: Pen,
    drawing: Pen,
    painting: Pen,
    dividing: Pen,
    valuing: Pen,
    /// The kind whose How shows.
    instructions: Option<Kind>,
    /// The kinds whose How showed by itself already, or need not: compared before; by their
    /// facts' keys (a kind's How is its topic's).
    told: HashSet<&'static str>,
    notice: Option<PracticeNotice>,
    /// The record changed since the store last got it.
    changed: bool,
    /// The app was hidden (or the sheet just opened) since the last use, and the time the store
    /// last got (`time.rs`).
    hidden: bool,
    time_saved: u64,
    /// The aligned fit for the worker, and how many were asked.
    fit_job: Option<FitJob>,
    fits: u64,
    /// A painted exercise's state: how much of the original shows, the area compared, the
    /// value scale's cell, the copy's pixels asked for, the eyedropper's comparison.
    painting_state: Painting,
    /// The pictures read (none: unreadable), a single shape's with its ground faded (S99), and
    /// those asked of the store.
    images: Vec<(ItemRef, Option<Arc<PictureImage>>)>,
    grounded: Vec<(ItemRef, Arc<PictureImage>)>,
    asked: HashSet<ItemRef>,
    /// The zoom the view had before Finish, which One more glides to (`view.rs`).
    zoom: Option<f32>,
    /// A lesson appended in place waiting for the store, and what it rolls back to (`append.rs`).
    appending: Option<Box<append::Appending>>,
}

impl PracticeSheet {
    pub(crate) fn new(opening: Practising, canvas: CanvasSize, pens: PracticePens) -> Self {
        let Practising {
            record,
            view,
            beside,
            told,
        } = opening;
        let sheet = Arc::new(view);
        let mut practice = Self {
            told,
            all_tools: record.is_copy(),
            hidden: true,
            time_saved: seconds(&record.spans),
            record,
            beside,
            end: None,
            sheet,
            in_hand: None,
            canvas,
            pens,
            clip: None,
            compared: None,
            aligned: false,
            construction: false,
            pen: Pen::Draw,
            drawing: Pen::Draw,
            painting: Pen::Big,
            dividing: Pen::Main,
            valuing: Pen::Middle,
            instructions: None,
            notice: None,
            changed: false,
            fit_job: None,
            fits: 0,
            painting_state: Painting::default(),
            images: Vec::new(),
            grounded: Vec::new(),
            asked: HashSet::new(),
            zoom: None,
            appending: None,
        };
        practice.shown();
        practice
    }

    /// The next picture of the session to read for the screen, once each.
    pub fn picture_request(&mut self) -> Option<ItemRef> {
        let item = (self.sheet.pictures.iter().flatten())
            .map(|picture| &picture.item)
            .find(|item| !self.asked.contains(*item))?
            .clone();
        self.asked.insert(item.clone());
        Some(item)
    }

    /// The store read `item`'s picture (`None`: it could not); one whose ground shows faded (a
    /// single shape's) faded once (S99).
    pub fn picture_read(&mut self, item: ItemRef, image: Option<PictureImage>) {
        self.images.retain(|(kept, _)| *kept != item);
        self.grounded.retain(|(kept, _)| *kept != item);
        let faded = (self.sheet.pictures.iter().zip(&self.sheet.kinds))
            .filter(|(_, kind)| kind.facts().grounded)
            .find_map(|(picture, _)| picture.as_ref().filter(|p| p.item == item));
        let grounded = faded
            .zip(image.as_ref())
            .and_then(|(picture, image)| grounded(picture, image));
        if let Some(grounded) = grounded {
            self.grounded.push((item.clone(), Arc::new(grounded)));
        }
        self.images.push((item, image.map(Arc::new)));
    }

    /// Its record as it stands.
    #[must_use]
    pub const fn record(&self) -> &Sheet {
        &self.record
    }

    /// The exercise shown, unless the session is done.
    fn current(&self) -> Option<usize> {
        let index = self.record.at();
        (self.record.finished().is_none() && index < self.sheet.pairs.len()).then_some(index)
    }

    fn pair(&self, index: usize) -> Option<&Pair> {
        self.sheet.pairs.get(index)
    }

    /// Whether exercise `index` is the last of its lesson (Next finishes it).
    fn ends(&self, index: usize) -> bool {
        let held = self.record.held();
        let end = held.map_or(0, |held| {
            held.first_exercise() + held.lesson.exercises.len()
        });
        index + 1 >= end.min(self.sheet.pairs.len())
    }

    fn kind(&self, index: usize) -> Option<Kind> {
        Some(self.record.laid(index)?.exercise.kind)
    }

    /// What exercise `index`'s kind's description says of it.
    fn facts(&self, index: usize) -> Option<&'static Facts> {
        Some(self.kind(index)?.facts())
    }

    /// What the shown exercise's kind's description says of it.
    fn shown_facts(&self) -> Option<&'static Facts> {
        self.facts(self.current()?)
    }

    /// Whether Aligned can be chosen for exercise `index`: its copy is laid over as lines, its
    /// original lines or a picture with an outline.
    fn aligned_offered(&self, index: usize) -> bool {
        let lines = self.facts(index).is_some_and(|f| f.aligned);
        let original = self.original(index);
        let outlined = (self.sheet.pictures.get(index))
            .is_some_and(|p| p.as_ref().is_some_and(|p| p.outline.is_some()));
        lines && (original.is_some_and(|o| o.lines().is_some()) || outlined)
    }

    /// An exercise came to show: its frame (a value scale's first cell) becomes the clip; lines
    /// compared before are corrected in blue, others drawn with the pen last chosen, painted
    /// exercises painted with the painting pen last chosen (value masses with the grey last
    /// chosen); its kind's How shows by itself the first time it comes up (never in a warm-up,
    /// and in the first lesson only at its first exercise);
    /// once the lesson is done, its end is kept.
    fn shown(&mut self) {
        self.take_in_hand();
        let current = self.current();
        let frame = current.map(|index| self.sheet.pairs[index].copy);
        self.clip = frame.and_then(|frame| self.region(&frame));
        let compared = current.is_some_and(|index| self.record.first(index).is_some());
        let kind = current.and_then(|index| self.kind(index));
        self.pen = match kind.map(|kind| kind.facts().tools) {
            Some(Tools::Greys) => self.valuing,
            Some(Tools::Colour | Tools::Cells | Tools::Painting | Tools::Full) => self.painting,
            Some(Tools::Dividing | Tools::Copying) => self.dividing,
            _ if compared => Pen::Correct,
            _ => self.drawing,
        };
        self.painted_shown();
        let at = current.zip(self.record.held()).and_then(|(index, held)| {
            let at = index.checked_sub(held.first_exercise())?;
            Some((at, held.lesson.warm_up()))
        });
        // The first lesson's How (shown in place of a kind's) comes by itself once, at its start.
        let quiet = at.is_some_and(|(at, warm_up)| at < warm_up || (at > 0 && !self.scored()));
        if let Some(kind) = kind.filter(|_| !quiet)
            && self.told.insert(kind.facts().key)
        {
            self.instructions = Some(kind);
        }
        self.ended();
    }

    /// `frame`'s region of the canvas.
    fn region(&self, frame: &Frame) -> Option<RegionMask> {
        let corners = frame.corners().map(|p| p.map(|v| v as f32));
        RegionMask::polygon(self.canvas, &corners)
    }

    /// What strokes and fills stay inside.
    pub(crate) fn frame_clip(&self) -> Option<&RegionMask> {
        self.clip.as_ref()
    }

    /// Pairs shown: up to the one shown, or all once the session is done.
    fn revealed(&self) -> usize {
        match self.current() {
            Some(index) => index + 1,
            None => self.sheet.pairs.len(),
        }
    }

    /// Applies `intent` at `now` (seconds since 1970 UTC); the view moves for Next. A fill is
    /// the session's (`fill_frame`).
    pub(crate) fn handle(
        &mut self,
        intent: PracticeIntent,
        artwork: &Artwork,
        (view, now): (&mut CanvasView, u64),
    ) -> Result<(), Refused> {
        match intent {
            PracticeIntent::Instructions(on) => {
                self.instructions = on
                    .then(|| self.current().and_then(|i| self.kind(i)))
                    .flatten();
            }
            PracticeIntent::Compare(on) => self.compare(on, artwork, now)?,
            PracticeIntent::Aligned(on) => {
                let offered = self.current().is_some_and(|i| self.aligned_offered(i));
                if on && !offered {
                    return Err(unavailable(Availability::NothingLeft));
                }
                self.aligned = on;
            }
            PracticeIntent::Construction(on) => {
                if !self.construction_offered() {
                    return Err(unavailable(Availability::NothingLeft));
                }
                self.construction = on;
            }
            PracticeIntent::Shown(share) => self.painting_state.shown = share.clamp(0.0, 1.0),
            PracticeIntent::Area(area) => self.choose_area(area)?,
            PracticeIntent::Grey(on) => self.grey(on)?,
            PracticeIntent::Pen(pen) => self.choose(pen)?,
            PracticeIntent::AllTools(on) => {
                // A long colour copy has no tools of its own (S104).
                if !on && self.record.is_copy() {
                    return Err(unavailable(Availability::NothingLeft));
                }
                self.all_tools = on;
            }
            PracticeIntent::Next(viewport) => self.next(view, &viewport, now)?,
            PracticeIntent::OneMore(_) => {
                // A lesson appended waits for the store before another may follow it.
                if self.current().is_some() || self.appending.is_some() {
                    return Err(unavailable(Availability::Running));
                }
            }
            PracticeIntent::Fill(_) => return Err(unavailable(Availability::NothingLeft)),
            PracticeIntent::Judge(outcome) => self.judge(outcome)?,
            PracticeIntent::Dismiss => self.notice = None,
        }
        Ok(())
    }

    /// The record for the store, once after each change (its time included); none while a
    /// lesson appended waits for the store (a record naming it must not be stored before it).
    pub fn save_request(&mut self) -> Option<Sheet> {
        if self.appending.is_some() {
            return None;
        }
        let changed = std::mem::take(&mut self.changed);
        changed.then(|| {
            self.time_saved = seconds(&self.record.spans);
            self.record.clone()
        })
    }
}

fn unavailable(availability: Availability) -> Refused {
    Refused::Unavailable(availability)
}
