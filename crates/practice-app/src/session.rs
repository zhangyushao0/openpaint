//! A practice sheet open on the canvas (D112): the session its artwork is drawn in and the
//! sheet's flow, kept in step. The sheet opens on its shown pair and its strokes keep the pen's
//! own samples (no StreamLine, QuickShape or guide); a pen landing outside the shown exercise's
//! empty frame draws nothing, every stroke and fill stays inside it (or a value scale's cell, the
//! session's bounds); a line or a painted exercise shown with its own tools is drawn with the
//! practice pen chosen instead of the toolbox's brush; a whole copy's layers follow its pen and
//! Compare, amended into the version shown; a fill beginning takes the comparison away. Its own
//! intents (`practise`) end what any edit ends first: the colour beside a colour's frame (a
//! combination's, a value scale's cell) fills it in one step, Next cuts the history to the
//! exercise shown. After each of them, and after a pen landing, the session's bounds, pen and
//! layers are set as the sheet has them (`held`). What the store and the workers answer the runtime
//! hands to `PracticeSheet` itself (`sheet_mut`).

use openpaint_app::{
    AppError, Availability, ContactPhase, Device, Intent, Session, StrokeIntent, Viewport,
};
use openpaint_core::{Artwork, StrokeStyle};
use openpaint_practice::{Givens, Lesson, Sheet};

use crate::layers::layered;
use crate::log::PracticeLog;
use crate::pens::PracticePens;
use crate::projection::{PracticeIntent, PracticeProjection};
use crate::refused::Refused;
use crate::sheet::{PracticeSheet, Practising};

/// A practice sheet open on the canvas: the session its artwork is drawn in and the sheet's
/// flow, kept in step.
#[derive(Debug)]
pub struct SheetSession {
    session: Session,
    sheet: PracticeSheet,
    /// The toolbox's style; strokes start with it unless the sheet's pen draws.
    brush: StrokeStyle,
}

impl SheetSession {
    /// The practice sheet `artwork` of `sheet` (`PracticeLog::practising`: its record, the sheet
    /// as drawn and what its lesson in hand is set beside), opening on its shown pair in
    /// `viewport`; `brush` is the toolbox's, `pens` the practice pens.
    #[must_use]
    pub fn open(
        artwork: Artwork,
        (brush, pens): (StrokeStyle, PracticePens),
        sheet: Practising,
        viewport: &Viewport,
    ) -> Self {
        let sheet = PracticeSheet::new(sheet, artwork.canvas(), pens);
        let mut session = Session::open(artwork, brush.clone());
        session.keep_raw();
        *session.view_mut() = sheet.opening_view(viewport);
        let mut open = Self {
            session,
            sheet,
            brush,
        };
        open.held();
        open
    }

    /// The session the sheet's artwork is drawn in.
    #[must_use]
    pub const fn session(&self) -> &Session {
        &self.session
    }

    /// For what the workers answer and the store asks; intents go through `dispatch`.
    pub const fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// The sheet's flow.
    #[must_use]
    pub const fn sheet(&self) -> &PracticeSheet {
        &self.sheet
    }

    /// The sheet's flow, to hand it what the store and the workers found.
    pub const fn sheet_mut(&mut self) -> &mut PracticeSheet {
        &mut self.sheet
    }

    /// One painting intent of the frame's: a pen landing the sheet refuses draws nothing; a fill
    /// beginning takes the comparison away; the sheet's layers are amended after.
    ///
    /// # Errors
    /// As `Session::dispatch`.
    pub fn dispatch(&mut self, intent: Intent) -> Result<(), AppError> {
        if let Intent::Stroke(_) = intent {
            // What a stroke ends, ended before the sheet answers, as the session does first.
            self.session.make_way(&intent);
            let draws = landing(&intent).is_none_or(|at| self.sheet.may_draw(at));
            self.session.set_bounds(self.sheet.frame_clip().cloned());
            // The stroke begins on the layer the sheet keeps for its pen.
            self.keep_layers();
            if !draws {
                return Ok(());
            }
        } else if matches!(intent, Intent::Fill(_)) {
            self.sheet.laid();
        }
        let result = self.session.dispatch(intent);
        self.keep_layers();
        result
    }

    /// Applies a practice intent at `now` (seconds since 1970 UTC, when a check is stored), after
    /// what any edit ends.
    ///
    /// # Errors
    /// `Unavailable` for Compare with nothing to compare, the blue pen before Compare, a pen the
    /// exercise does not offer, a long colour copy's own tools (it has none, S104), a fill but on
    /// a colour's frame (or of a hidden or locked layer, which the notice says), a result chosen
    /// where the app judges, before Compare or twice, and One more before the lesson is done;
    /// `Core` when the artwork refuses the fill.
    pub fn practise(&mut self, intent: PracticeIntent, now: u64) -> Result<(), Refused> {
        self.session.make_way_for_step();
        let result = self.apply(intent, now);
        self.held();
        result
    }

    /// The toolbox's style for the strokes to come (the sheet's pen draws instead while it does).
    pub fn set_brush(&mut self, brush: StrokeStyle) {
        self.brush = brush;
        self.restyle();
    }

    /// The diameter of the pen the sheet draws with instead of the brush, while it does: the
    /// hover outline's.
    #[must_use]
    pub fn pen(&self) -> Option<f32> {
        self.sheet.pen_style(&self.brush).map(|style| style.size())
    }

    /// The sheet for one UI frame.
    #[must_use]
    pub fn projection(&self) -> PracticeProjection {
        self.sheet.projection(self.session.artwork(), &self.brush)
    }

    /// One more in place (S121c): `lesson` and `givens`, what it prints, appended where the sheet
    /// takes them (`PracticeSheet::takes`; `today` whether the sheet was started on today's local
    /// day by `log`), the view gliding to its first pair in `viewport`, undo and redo reaching only
    /// the exercise shown. Returns the record and the givens for the store; none when the lesson
    /// goes on a new sheet.
    pub fn append_lesson(
        &mut self,
        (lesson, givens): (Lesson, Givens),
        log: &PracticeLog,
        viewport: &Viewport,
    ) -> Option<(Sheet, Givens)> {
        let today = log.is_today(self.sheet.record());
        self.session.make_way_for_step();
        let appended = self.sheet.takes(lesson.exercises.len(), today).then(|| {
            let view = (self.session.view_mut(), viewport);
            let appended = self.sheet.append((lesson, givens), log, view);
            self.session.keep_only_current();
            appended
        });
        self.held();
        appended
    }

    /// The store wrote the lesson appended to sheet `id` (`stored`), or did not: then the sheet
    /// rolls it back (`PracticeSheet::appended`).
    pub fn append_stored(&mut self, id: u64, stored: bool) {
        self.session.make_way_for_step();
        self.sheet.appended(id, stored, self.session.view_mut());
        self.held();
    }

    /// End (`crate::pomodoro::TimerIntent::End`) at `now` of a block begun at `since`, the view
    /// fitting the rows drawn in it within `viewport`; undo and redo then reach nothing before
    /// it. Returns the record as it stands.
    ///
    /// # Errors
    /// `Unavailable` on a colour copy, or while a lesson appended waits for the store.
    pub(crate) fn end_block(
        &mut self,
        since: u64,
        (now, viewport): (u64, &Viewport),
    ) -> Result<Sheet, Refused> {
        self.session.make_way_for_step();
        let artwork = self.session.artwork().clone();
        let view = self.session.view_mut();
        let record = self
            .sheet
            .end_block((&artwork, view), since, (now, viewport));
        if record.is_ok() {
            self.session.keep_only_current();
        }
        self.held();
        record
    }

    /// Whether an exercise of a block begun at `since` was begun on the sheet (drawn in or
    /// compared; `PracticeSheet::begun_since`).
    pub(crate) fn begun_since(&self, since: u64) -> bool {
        self.sheet.begun_since(self.session.artwork(), since)
    }

    /// `intent` at `now`: a fill recorded as one step, Next cutting the history to the exercise
    /// it shows, anything else the sheet's own.
    fn apply(&mut self, intent: PracticeIntent, now: u64) -> Result<(), Refused> {
        let artwork = self.session.artwork().clone();
        match intent {
            PracticeIntent::Fill(_) if self.session.active().is_some() => {
                Err(Refused::Unavailable(Availability::StrokeInProgress))
            }
            PracticeIntent::Fill(color) => {
                let region = self.sheet.fill_region(&artwork, self.session.selection())?;
                let filled = artwork.fill_region(region, self.sheet.fill_colour(color))?;
                self.session.record(filled.artwork);
                Ok(())
            }
            PracticeIntent::Next(_) => {
                let view = (self.session.view_mut(), now);
                self.sheet.handle(intent, &artwork, view)?;
                // Undo and redo reach only the exercise shown: never an earlier one, whose
                // Compare's record stands.
                self.session.keep_only_current();
                Ok(())
            }
            intent => (self.sheet).handle(intent, &artwork, (self.session.view_mut(), now)),
        }
    }

    /// After a step of the sheet's own: the session's bounds, pen and layers as the sheet has
    /// them.
    fn held(&mut self) {
        self.session.set_bounds(self.sheet.frame_clip().cloned());
        self.restyle();
        self.keep_layers();
    }

    /// The next stroke's style: the sheet's pen, else the toolbox's brush.
    fn restyle(&mut self) {
        let pen = self.sheet.pen_style(&self.brush);
        self.session
            .set_brush(pen.unwrap_or_else(|| self.brush.clone()));
    }

    /// A whole copy's division and line drawing layers as the pen and Compare want them
    /// (`layers.rs`), amended into the version shown; a sheet with no room for them stays as it
    /// is. Never while a stroke is drawn.
    fn keep_layers(&mut self) {
        if self.session.active().is_some() {
            return;
        }
        let wanted = self.sheet.layering();
        let layered = wanted.and_then(|layering| layered(self.session.artwork(), layering).ok());
        if let Some(artwork) = layered {
            self.session.amend(artwork);
        }
    }
}

/// Where a pen or the eraser lands, when `intent` is its landing with a sample.
fn landing(intent: &Intent) -> Option<[f32; 2]> {
    let Intent::Stroke(StrokeIntent::Contact(contact)) = intent else {
        return None;
    };
    let lands = contact.phase == ContactPhase::Down
        && matches!(contact.device, Device::Pen | Device::Eraser);
    contact
        .samples
        .first()
        .filter(|_| lands)
        .map(|first| first.position)
}
