//! The Pomodoro from the log (S124, `crate::pomodoro`): what the runtime asks of the timer and
//! hands it, all through the log that holds it. Each frame the runtime ticks it with its clock and
//! the sheet in front (`tick`, before the UI pass, so the pass shows that tick's state), shows it
//! (`timer`), applies the screens' timer intents (`time`, with the open sheet's session), lays the
//! chain after a Next (`chains`), asks for the frame it wants (`wake`), keeps the screen on
//! (`screen_on`), sounds the chime (`take_chime`) and stores what changed (`timer_request`).

use openpaint_app::Session;
use openpaint_practice::{Blocks, Sheet, TimerWrite};

use super::PracticeLog;
use crate::pomodoro::{Pomodoro, Timed, TimerIntent, TimerShown};
use crate::refused::Refused;

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

    /// The timer as one UI frame shows it, at the last tick.
    #[must_use]
    pub fn timer(&self) -> TimerShown {
        // S124b
        let timing = self.pomodoro.timing();
        TimerShown {
            length: timing.length,
            chime: timing.chime,
            sealed: 0,
            block: None,
        }
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
        // S124b
        let _ = (intent, session);
        Err(Refused::Unavailable(
            openpaint_app::Availability::NothingLeft,
        ))
    }

    /// Whether the lesson in hand on `session`'s sheet has just ended while a block counts with
    /// time left: the runtime then lays the next lesson as One more lays it (the chain).
    #[must_use]
    pub fn chains(&self, session: &Session) -> bool {
        // S124b
        let _ = session;
        false
    }

    /// Seconds after the last tick at which the timer wants a frame: the turn of the minute shown
    /// or the end of the block or of the break; none while nothing counts.
    #[must_use]
    pub fn wake(&self) -> Option<u64> {
        // S124b
        None
    }

    /// Whether the screen stays on: the block or its break counts.
    #[must_use]
    pub fn screen_on(&self) -> bool {
        // S124b
        false
    }

    /// Whether the break's chime sounds now: once, at its end, the switch on.
    pub fn take_chime(&mut self) -> bool {
        // S124b
        false
    }

    /// The timer's records for the store, once after each change (`practice_store::save_timer`).
    pub fn timer_request(&mut self) -> Option<TimerWrite> {
        // S124b
        let _ = self.pomodoro.now();
        None
    }
}
