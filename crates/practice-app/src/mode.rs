//! The practice sheet as the session's mode (`openpaint_app::Mode`, D112): a sheet opens on its
//! shown pair, its strokes keep the pen's own samples (no StreamLine, QuickShape or guide), the
//! pen draws only in the shown exercise's empty frame and every stroke and fill stays inside it, a
//! line or a painted exercise shown with its own tools is drawn with the practice pen chosen
//! instead of the toolbox's brush, a whole copy's layers follow its pen and Compare, and a fill
//! beginning takes the comparison away. Its own intents reach it through `Session::lend`
//! (`practise`): the colour beside a colour's frame (a combination's, a value scale's cell) fills
//! it in one step, Next cuts the history to the exercise shown. Everything else the runtime asks
//! of the sheet it asks of `PracticeSheet` itself (`of`, `of_mut`).

use openpaint_app::{Availability, Lent, Mode, Session, Viewport};
use openpaint_core::{Artwork, RegionMask, StrokeStyle};
use openpaint_practice::{Givens, Lesson, Sheet};

use crate::layers::layered;
use crate::log::PracticeLog;
use crate::pens::PracticePens;
use crate::projection::{PracticeIntent, PracticeProjection};
use crate::refused::Refused;
use crate::sheet::{PracticeSheet, Practising};

impl Mode for PracticeSheet {
    fn raw(&self) -> bool {
        true
    }

    /// One landing inside the frame takes a comparison away; one outside is refused, the
    /// sheet's notice saying so.
    fn lands(&mut self, at: [f32; 2]) -> bool {
        self.may_draw(at)
    }

    /// The shown exercise's empty frame, or a value scale's cell.
    fn clip(&self) -> Option<&RegionMask> {
        self.frame_clip()
    }

    fn style(&self, brush: &StrokeStyle) -> Option<StrokeStyle> {
        self.pen_style(brush)
    }

    /// A whole copy's division and line drawing layers as the pen and Compare want them
    /// (`layers.rs`); a sheet with no room for them stays as it is.
    fn amended(&self, current: &Artwork) -> Option<Artwork> {
        layered(current, self.layering()?).ok()
    }

    fn laid(&mut self) {
        // The sheet's own (`compare.rs`): the comparison goes.
        PracticeSheet::laid(self);
    }
}

impl PracticeSheet {
    /// The sheet the session holds, if it holds one.
    #[must_use]
    pub fn of(session: &Session) -> Option<&Self> {
        session.mode::<Self>()
    }

    /// The sheet the session holds, to hand it what the store and the workers found.
    pub fn of_mut(session: &mut Session) -> Option<&mut Self> {
        session.mode_mut::<Self>()
    }
}

/// The practice sheet `artwork` of `sheet` (its session's record, the finished sessions of its
/// stage before it and the sheet as drawn), opening on its shown pair in `viewport`; `brush` is
/// the toolbox's, `pens` the practice pens.
#[must_use]
pub fn open_sheet(
    artwork: Artwork,
    (brush, pens): (StrokeStyle, PracticePens),
    sheet: Practising,
    viewport: &Viewport,
) -> Session {
    let practice = PracticeSheet::new(sheet, artwork.canvas(), pens);
    let view = practice.opening_view(viewport);
    Session::with_mode(artwork, brush, Box::new(practice), view)
}

/// Applies a practice intent to the sheet `session` holds at `now` (seconds since 1970 UTC, when
/// a check is stored).
///
/// # Errors
/// `Unavailable` off a practice sheet, for Compare with nothing to compare, the blue pen before
/// Compare, a pen the exercise does not offer, a long colour copy's own tools (it has none,
/// S104), a fill but on a colour's frame (or of a hidden or locked layer, which the notice says)
/// and One more before the session is done; `Core` when the artwork refuses the fill.
pub fn practise(session: &mut Session, intent: PracticeIntent, now: u64) -> Result<(), Refused> {
    session
        .lend::<PracticeSheet, _>(|sheet, lent| sheet.apply(intent, lent, now))
        .unwrap_or(Err(Refused::Unavailable(Availability::NothingLeft)))
}

/// One more in place (S121c): `lesson` and `givens`, what it prints, appended to the sheet
/// `session` holds when it takes them (`PracticeSheet::takes`; `today` whether the sheet was
/// started on today's local day by `log`), the view gliding to its first pair in `viewport`, undo
/// and redo reaching only the exercise shown. Returns the record and the givens for the store;
/// none when the lesson goes on a new sheet.
pub fn append_lesson(
    session: &mut Session,
    (lesson, givens): (Lesson, Givens),
    log: &PracticeLog,
    viewport: &Viewport,
) -> Option<(Sheet, Givens)> {
    let today = PracticeSheet::of(session).is_some_and(|sheet| log.is_today(sheet.record()));
    session
        .lend::<PracticeSheet, _>(|sheet, lent| {
            if !sheet.takes(lesson.exercises.len(), today) {
                return None;
            }
            let appended = sheet.append((lesson, givens), log, (lent.view(), viewport));
            lent.keep_only_current();
            Some(appended)
        })
        .flatten()
}

/// The store wrote the lesson appended to sheet `id` (`stored`), or did not: then the sheet
/// `session` holds rolls it back (`PracticeSheet::appended`).
pub fn append_stored(session: &mut Session, id: u64, stored: bool) {
    session.lend::<PracticeSheet, _>(|sheet, lent| sheet.appended(id, stored, lent.view()));
}

impl PracticeProjection {
    /// The open practice sheet for one UI frame, if `session` holds one.
    #[must_use]
    pub fn of(session: &Session) -> Option<Self> {
        let sheet = PracticeSheet::of(session)?;
        Some(sheet.projection(session.artwork(), session.brush()))
    }
}

impl PracticeSheet {
    /// `intent` at `now` with what the session lends: a fill recorded as one step, Next cutting
    /// the history to the exercise it shows, anything else the sheet's own.
    fn apply(
        &mut self,
        intent: PracticeIntent,
        lent: &mut Lent<'_>,
        now: u64,
    ) -> Result<(), Refused> {
        let artwork = lent.artwork().clone();
        match intent {
            PracticeIntent::Fill(_) if lent.drawing() => {
                Err(Refused::Unavailable(Availability::StrokeInProgress))
            }
            PracticeIntent::Fill(color) => {
                let region = self.fill_region(&artwork, lent.selection())?;
                let filled = artwork.fill_region(region, self.fill_colour(color))?;
                lent.record(filled.artwork);
                Ok(())
            }
            PracticeIntent::Next(_) => {
                self.handle(intent, &artwork, (lent.view(), now))?;
                // Undo and redo reach only the exercise shown: never an earlier one, whose
                // Compare's record stands.
                lent.keep_only_current();
                Ok(())
            }
            intent => self.handle(intent, &artwork, (lent.view(), now)),
        }
    }
}
