//! The Pomodoro from the log (S124, `crate::pomodoro`): what the runtime asks of the timer and
//! hands it, all through the log that holds it. Each frame the runtime ticks it with its clock and
//! the sheet in front (`tick`, before the UI pass, so the pass shows that tick's state), shows it
//! (`timer`), applies the screens' timer intents (`time`, with the open sheet's session), lays the
//! chain after a Next (`chains`), asks for the frame it wants (`wake`), keeps the screen on
//! (`screen_on`), sounds the chime (`take_chime`) and stores what changed (`timer_request`).

use openpaint_app::{Availability, Session};
use openpaint_practice::{Blocks, Sheet, TimerWrite};

use super::PracticeLog;
use crate::pomodoro::{Pomodoro, Timed, TimerIntent, TimerShown};
use crate::refused::Refused;
use crate::sheet::{PracticeSheet, begun_on, end_block};

impl PracticeLog {
    /// What the store kept of the Pomodoro (`practice_store::blocks`), as the app starts: a block
    /// under way comes back held, its seconds left as kept.
    pub fn set_blocks(&mut self, blocks: Blocks) {
        self.pomodoro = Pomodoro::of(blocks);
        self.booked();
    }

    /// The clock at `now` (seconds since 1970 UTC; `set_clock` first), `front` the record of the
    /// practice sheet in front (none: the home, the book, the path, a sheet opening, the app
    /// hidden): the block counts down only while a day's or a test's sheet is in front.
    pub fn tick(&mut self, now: u64, front: Option<&Sheet>) {
        let shown = front.is_some_and(|sheet| !sheet.is_copy());
        self.pomodoro.tick(now, (self.today, shown));
    }

    /// The app hidden at `now` (`set_clock` first; Android `onPause`): the block and its break
    /// held as the home holds them, the block's end kept for the sheet's return.
    pub fn hidden(&mut self, now: u64) {
        self.pomodoro.hidden(now);
    }

    /// The timer as one UI frame shows it, at the last tick.
    #[must_use]
    pub fn timer(&self) -> TimerShown {
        self.pomodoro.shown()
    }

    /// `intent` at the last tick, the open sheet's `session` when a canvas shows one: what the
    /// runtime does next.
    ///
    /// # Errors
    /// `Unavailable` for an intent the timer's state does not take: Pause, Continue and Stop with
    /// no block under way, End before the time is up or off a sheet, Another but on a block's end.
    pub fn time(
        &mut self,
        intent: TimerIntent,
        session: Option<&mut Session>,
    ) -> Result<Timed, Refused> {
        match intent {
            TimerIntent::Length(length) => self.pomodoro.choose(length),
            TimerIntent::Chime(on) => self.pomodoro.chime_on(on),
            TimerIntent::Start => {
                self.pomodoro.begin();
                return Ok(Timed::Lesson);
            }
            TimerIntent::Pause => self.pomodoro.pause(true)?,
            TimerIntent::Resume => self.pomodoro.pause(false)?,
            TimerIntent::Stop => self.pomodoro.stop()?,
            TimerIntent::End(viewport) => {
                let block = self.pomodoro.up().ok_or(NOT_NOW)?;
                let session = session.ok_or(NOT_NOW)?;
                if !self.begun_in(session, block.started) {
                    // Nothing begun in it: End leaves it as Stop timer does.
                    self.pomodoro.stop()?;
                    return Ok(Timed::Done);
                }
                let now = self.pomodoro.now();
                let record = end_block(session, block.started, (now, &viewport))?;
                // A lesson taken off leaves the one before it in hand: set beside the log again.
                let beside = self.beside(&record);
                if let Some(sheet) = PracticeSheet::of_mut(session) {
                    sheet.set_beside(beside);
                }
                self.pomodoro.complete();
                // The log takes the sheet cut: its book, its seals and the block's end made again.
                self.keep(record);
            }
            TimerIntent::Another(viewport) => {
                if !self.pomodoro.ended() {
                    return Err(NOT_NOW);
                }
                self.pomodoro.begin();
                return Ok(Timed::OneMore(viewport));
            }
        }
        Ok(Timed::Done)
    }

    /// Whether an exercise of a block begun at `since` was begun: on `session`'s sheet (drawn in
    /// or compared), or a lesson finished in it on another sheet (one the chain left).
    fn begun_in(&self, session: &Session, since: u64) -> bool {
        let here = PracticeSheet::of(session).map(|sheet| sheet.record().id);
        let elsewhere = (self.sheets.iter())
            .filter(|sheet| Some(sheet.id) != here && !sheet.is_copy())
            .flat_map(|sheet| &sheet.lessons)
            .any(|lesson| lesson.finished.is_some_and(|finished| finished >= since));
        elsewhere || begun_on(session, since)
    }

    /// Whether the lesson in hand on `session`'s sheet has just ended while a block counts with
    /// time left: the runtime then lays the next lesson as One more lays it (the chain).
    #[must_use]
    pub fn chains(&self, session: &Session) -> bool {
        let Some(record) = PracticeSheet::of(session).map(PracticeSheet::record) else {
            return false;
        };
        self.pomodoro.counts_on() && !record.is_copy() && record.finished().is_some()
    }

    /// Seconds after the last tick at which the timer wants a frame: the turn of the minute shown
    /// or the end of the block or of the break; none while nothing counts.
    #[must_use]
    pub fn wake(&self) -> Option<u64> {
        self.pomodoro.wake()
    }

    /// Whether the screen stays on: the block or its break counts.
    #[must_use]
    pub fn screen_on(&self) -> bool {
        self.pomodoro.screen_on()
    }

    /// Whether the break's chime sounds now: once, at its end, the switch on.
    pub fn take_chime(&mut self) -> bool {
        self.pomodoro.take_chime()
    }

    /// For debug builds' test chore only (`practice-test start pomodoro`): a block of the length
    /// chosen begun at the last tick with only `seconds` left, so the time's end comes within a
    /// minute; false while one is under way (it stays as it is).
    pub fn begin_test_block(&mut self, seconds: u32) -> bool {
        self.pomodoro.begin_test(seconds)
    }

    /// For debug builds' test chore only (`practice-test end`): the blocks begun at or after
    /// `since` forgotten, as the store removes them (`practice_store::forget_blocks`).
    pub fn forget_blocks(&mut self, since: u64) {
        self.pomodoro.forget(since);
        self.booked();
    }

    /// The timer's records for the store, once after each change (`practice_store::save_timer`).
    pub fn timer_request(&mut self) -> Option<TimerWrite> {
        self.pomodoro.request()
    }
}

/// An intent the timer's state does not take now.
const NOT_NOW: Refused = Refused::Unavailable(Availability::NothingLeft);
