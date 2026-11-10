//! The open practice sheet's flow (`practice.rs`): which exercise shows, where the pen may draw
//! (strokes and fills only in its empty frame, or a value scale's cell, clipped to it), the
//! exercise's own tools or the full interface, the pen drawn exercises are drawn with (a single
//! line, combined lines, a single shape) or painted ones painted with (a colour combination, a
//! value scale), Compare and its overlay (`compare.rs`), a painted copy's reading and the
//! eyedropper (`read.rs`, a reading's frames in grey on asking), Next
//! (the view glides to the next pair at its zoom,
//! `view.rs`; after the last the session ends over the whole sheet), the instructions
//! (by themselves the first time a family comes up), the summary and the notices; the session's
//! pictures as the store reads them. A long colour copy is a sheet of one pair, the full
//! interface always showing (painted with the toolbox's own brushes, S104) and no Next. The
//! session record changes here and goes to the store whenever it did (`save_request`).

use std::collections::HashSet;
use std::sync::Arc;

use openpaint_core::{Artwork, CanvasSize, RegionMask, StrokeStyle};
use openpaint_practice::{
    Facts, Family, Frame, ItemRef, Pair, Sheet, Tally, Tools, Work, seconds, summary,
};

use crate::pens::PracticePens;
use crate::pictures::{FitJob, PictureImage, grounded};
use crate::projection::{Pen, PracticeIntent, PracticeNotice, PracticeProjection, SheetView};
use crate::refused::Refused;
use openpaint_app::Availability;
use openpaint_app::CanvasView;

/// A practice sheet opening: its session's record, the finished sessions of its stage before it
/// (for the summary), the sheet as the screen draws it, and what every other record adds up to
/// (for the seals its finish stamps).
pub type Practising = (Sheet, Vec<Sheet>, SheetView, Tally);

#[path = "checks.rs"]
mod checks;
#[path = "compare.rs"]
mod compare;
#[path = "overlay.rs"]
mod overlay;
#[path = "sheet_pens.rs"]
mod pens;
#[path = "pick.rs"]
mod pick;
#[path = "read.rs"]
mod read;
#[path = "time.rs"]
mod time;
#[path = "view.rs"]
mod view;

pub(crate) use pens::{CopyLayer, Layering};
pub use pick::PracticePick;
pub use read::ReadJob;
pub use time::Finished;
pub use view::PRACTICE_ROOM;

use compare::Compared;
use read::Painting;

#[derive(Debug)]
pub struct PracticeSheet {
    record: Sheet,
    /// The finished sessions of its stage before it, for the summary.
    before: Vec<Sheet>,
    sheet: Arc<SheetView>,
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
    instructions: Option<Family>,
    /// Whether each family's instructions (`Family::ALL`'s order) showed by themselves already,
    /// or need not: a family compared before or in the warm-up.
    told: [bool; Family::ALL.len()],
    notice: Option<PracticeNotice>,
    /// The record changed since the store last got it.
    changed: bool,
    /// The app was hidden (or the sheet just opened) since the last use, the time the store last
    /// got, and what every other record adds up to (`time.rs`).
    hidden: bool,
    time_saved: u64,
    others: Tally,
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
}

impl PracticeSheet {
    pub(crate) fn new(
        (record, before, view, others): Practising,
        canvas: CanvasSize,
        pens: PracticePens,
    ) -> Self {
        let sheet = Arc::new(view);
        // Told: compared before or in this session, or warmed up with (only kinds met are); a
        // drill's How shows first every time it opens (S91).
        let done = |family: Family| {
            let compared = before.iter().chain([&record]).any(|session| {
                (session.exercises())
                    .any(|laid| laid.first.is_some() && laid.exercise.kind.family() == family)
            });
            let warmed = (record.exercises().take(record.warm_up()))
                .any(|laid| laid.exercise.kind.family() == family);
            (compared || warmed) && record.drilled() != Some(family)
        };
        let mut practice = Self {
            told: Family::ALL.map(done),
            all_tools: record.is_copy(),
            hidden: true,
            time_saved: seconds(&record.spans),
            others,
            record,
            before,
            sheet,
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
        let faded = (self.sheet.pictures.iter().zip(&self.sheet.families))
            .filter(|(_, family)| family.facts().grounded)
            .find_map(|(picture, _)| picture.as_ref().filter(|p| p.item == item));
        let grounded = faded
            .zip(image.as_ref())
            .and_then(|(picture, image)| grounded(picture, image));
        if let Some(grounded) = grounded {
            self.grounded.push((item.clone(), Arc::new(grounded)));
        }
        self.images.push((item, image.map(Arc::new)));
    }

    /// The kind a drill drills (One more starts another of it); none on a session's sheet.
    pub fn drill(&self) -> Option<Family> {
        self.record.drilled()
    }

    /// The exercise shown, unless the session is done.
    fn current(&self) -> Option<usize> {
        let index = self.record.at();
        (self.record.finished().is_none() && index < self.sheet.pairs.len()).then_some(index)
    }

    fn pair(&self, index: usize) -> Option<&Pair> {
        self.sheet.pairs.get(index)
    }

    fn family(&self, index: usize) -> Option<Family> {
        let exercise = &self.record.laid(index)?.exercise;
        Some(exercise.kind.family())
    }

    /// What exercise `index`'s family's description says of it.
    fn facts(&self, index: usize) -> Option<&'static Facts> {
        Some(self.family(index)?.facts())
    }

    /// What the shown exercise's family's description says of it.
    fn shown_facts(&self) -> Option<&'static Facts> {
        self.facts(self.current()?)
    }

    /// Whether Aligned can be chosen for exercise `index`: its copy is laid over as lines, its
    /// original lines or a picture with an outline.
    fn aligned_offered(&self, index: usize) -> bool {
        let lines = self.facts(index).is_some_and(|f| f.work == Work::Lines);
        let original = self.sheet.originals.get(index);
        let outlined = (self.sheet.pictures.get(index))
            .is_some_and(|p| p.as_ref().is_some_and(|p| p.outline.is_some()));
        lines && (original.is_some_and(|o| o.lines().is_some()) || outlined)
    }

    /// An exercise came to show: its frame (a value scale's first cell) becomes the clip; lines
    /// compared before are corrected in blue, others drawn with the pen last chosen, painted
    /// exercises painted with the painting pen last chosen (value masses with the grey last
    /// chosen); its family's instructions show by themselves the first time.
    fn shown(&mut self) {
        let frame = self.current().map(|index| self.sheet.pairs[index].copy);
        self.clip = frame.and_then(|frame| self.region(&frame));
        let compared = (self.current()).is_some_and(|index| self.record.first(index).is_some());
        let family = self.current().and_then(|index| self.family(index));
        self.pen = match family.map(|family| family.facts().tools) {
            Some(Tools::Greys) => self.valuing,
            Some(Tools::Colour | Tools::Cells | Tools::Painting | Tools::Full) => self.painting,
            Some(Tools::Dividing | Tools::Copying) => self.dividing,
            _ if compared => Pen::Correct,
            _ => self.drawing,
        };
        self.painted_shown();
        if let Some(family) = family {
            let at = Family::ALL.iter().position(|f| *f == family).unwrap_or(0);
            if !self.told[at] {
                self.told[at] = true;
                self.instructions = Some(family);
            }
        }
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
                    .then(|| self.current().and_then(|i| self.family(i)))
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
            PracticeIntent::OneMore => {
                if self.current().is_some() {
                    return Err(unavailable(Availability::Running));
                }
            }
            PracticeIntent::Fill(_) => return Err(unavailable(Availability::NothingLeft)),
            PracticeIntent::Dismiss => self.notice = None,
        }
        Ok(())
    }

    /// The record for the store, once after each change (its time included).
    pub fn save_request(&mut self) -> Option<Sheet> {
        let changed = std::mem::take(&mut self.changed);
        changed.then(|| {
            self.time_saved = seconds(&self.record.spans);
            self.record.clone()
        })
    }

    pub(crate) fn projection(&self, artwork: &Artwork, brush: &StrokeStyle) -> PracticeProjection {
        let current = self.current();
        let result = current.and_then(|index| self.record.first(index));
        let compare = self.compare_offered(current, artwork);
        let place = current.map_or((0, 0), |index| self.record.place(index));
        let before: Vec<&Sheet> = self.before.iter().collect();
        let pen = self.pen_style(brush).map(|style| (self.pen, style.size()));
        let painted = self.painted_overlay();
        let (overlay, card, greys) = current
            .and_then(|_| self.overlay())
            .map_or((None, Vec::new(), None), |(marks, card, greys)| {
                (Some(marks), card, greys)
            });
        PracticeProjection {
            sheet: Arc::clone(&self.sheet),
            revealed: self.revealed(),
            stage: self.record.stage(),
            copy: self.record.is_copy(),
            number: self.sheet.number,
            drill: self.record.drilled(),
            current,
            family: current.and_then(|index| self.family(index)),
            warming_up: current.is_some_and(|index| self.record.warming_up(index)),
            place,
            last: current.is_some_and(|index| index + 1 == self.sheet.pairs.len()),
            compare,
            comparing: self.compared.is_some(),
            aligned: self.aligned,
            construction: self.construction && self.construction_offered(),
            all_tools: self.all_tools,
            pen,
            result: result.cloned(),
            overlay,
            card,
            greys,
            instructions: self.instructions,
            summary: current.is_none().then(|| summary(&self.record, &before)),
            finished: self.finished(),
            notice: self.notice,
            aligned_offered: current.is_some_and(|index| self.aligned_offered(index)),
            fitting: self.fitting(),
            images: self.images.clone(),
            grounded: self.grounded.clone(),
            cell: self.cell(),
            probe: self.probe(),
            compared: (self.compared.as_ref()).map(|compared| compared.shown().clone()),
            shown: painted.0,
            area: painted.1,
            grey: painted.3,
        }
    }
}

fn unavailable(availability: Availability) -> Refused {
    Refused::Unavailable(availability)
}
