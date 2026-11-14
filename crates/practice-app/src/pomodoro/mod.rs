//! The Pomodoro (S124; its records `openpaint_practice::Blocks`): one tap starts a timed block of
//! lessons, and after that nothing asks "one more?" until the time is up. The timer is the
//! practice log's (`PracticeLog`, app-wide like the toolbox: it outlives a sheet, a sheet left and
//! a new sheet the chain opens), its clock the one the runtime hands in each frame
//! (`PracticeLog::tick`, seconds since 1970 UTC as everywhere in the flow), so a test drives it
//! with any clock it likes. The runtime asks the log (`log/timer.rs`) and does what it answers
//! (`Timed`); the screens read `TimerShown` and emit `TimerIntent`.
//!
//! The states and their rules, which the flow, the screens and the runtime keep alike:
//! - No block: the home's row offers 15, 25 and 40 minutes (the one chosen ringed and kept,
//!   `Timing`; 25 at first) and Start timer.
//! - Under way, counting: a day's or a test's sheet is in front (`tick`'s `front`), the user has
//!   not paused it, seconds are left. It counts down by the clock handed in, never below 0.
//! - Under way, held: no such sheet in front (the home, the book, the path, a colour copy, a
//!   sheet opening, the app hidden: the runtime ticks with none on `leave`) or the bar's Pause.
//!   It counts on when the sheet is in front again (and Continue, if the user paused it). A held
//!   block whose local day is past at a tick is dropped: nothing kept, no seal.
//! - Time up (no second left): the ring red and Time in the bar; nothing interrupts the stroke;
//!   the exercise in hand may be finished and compared; Next reads End (`TimerIntent::End`). The
//!   projection a frame's UI pass reads is that frame's tick's, so a Next tapped as the time runs
//!   out is laid out as End: the block ends there.
//! - The chain: while a block counts with time left, a lesson's last exercise passed (Next) ends
//!   the lesson as ever, and the runtime at once lays the next lesson as One more would
//!   (`PracticeLog::chains`; `End::next`: the path's next after this lesson counts, or the same
//!   skill again after a lesson of a unit past the learner's) on the next row of the day's sheet,
//!   the view gliding to its first pair, or on a new sheet when it does not fit. No end card shows
//!   between (the screens hide a lesson's end while a block runs). A kind's How still comes once.
//! - End: the lesson in hand ends with the exercises begun (the one in hand stays; those after it
//!   are taken off the sheet, its record and its stored givens: `PracticeSheet::trim_request`,
//!   `practice_store::trim`), finished at the tick; the view fits the rows drawn in the block; the
//!   block is completed (`TimerWrite::completed`: its seal); its end shows (`BlockEnd`) and the
//!   break of five minutes (`BREAK`) counts while the sheet is in front.
//! - The break: its ring counts down with the minutes left; at its end one soft chime (when the
//!   switch is on, `Timing::chime`; `PracticeLog::take_chime`, once) and the ring complete,
//!   nothing else. Another pomodoro works at once, during the break or after; That's all for today
//!   (or Home) leaves the block's end, which ends the break silently. The break is never stored.
//! - Stop timer (under way or time up): the block goes, nothing kept, no seal; the lesson in hand
//!   goes on untimed, as any lesson.
//!
//! Frames: the app draws none at rest; the timer wants one at the turn of each minute shown and
//! at the end of the block or the break (`PracticeLog::wake`), never every second. The screen
//! stays on while the block or the break counts (`PracticeLog::screen_on`). What changed goes to
//! the store at each transition and once a minute while it counts (`PracticeLog::timer_request`).

use openpaint_app::Viewport;
use openpaint_practice::{Block, Blocks, Length};

mod shown;

pub use shown::{BlockEnd, BlockShown, TimerShown};

/// What the user asks of the timer: on the home's row, the bar's ring, the block's end and the
/// practice options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerIntent {
    /// The home's row: a length chosen, ringed and kept for the next block.
    Length(Length),
    /// The home's row: Start timer, a block of the length chosen begins and the lesson Start
    /// would start opens (`Timed::Lesson`); with a block under way (the row's Continue) its lesson
    /// opens again.
    Start,
    /// The bar's ring: Pause, and Continue after it.
    Pause,
    Resume,
    /// The bar's ring: Stop timer. The block goes with no seal; the lessons drawn stay.
    Stop,
    /// The bar's End once the time is up: the block ends there (see the module), the view fitting
    /// the rows drawn in it within `Viewport`.
    End(Viewport),
    /// The block's end: Another pomodoro, at once, during the break or after: a new block of the
    /// length chosen and the next lesson laid as One more lays it (`Timed::OneMore`).
    Another(Viewport),
    /// The practice options' switch: the break's end chimes (true) or not.
    Chime(bool),
}

/// What the runtime does after a timer intent the log took.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timed {
    /// Nothing more.
    Done,
    /// From the home: the lesson under way opened again (its sheet), else the one Start starts
    /// (`OptionsIntent::Lesson(Start::Next)`), onto the day's sheet where it takes it.
    Lesson,
    /// On the open sheet: the next lesson laid as One more lays it (in place, the view gliding to
    /// it within the viewport; else on a new sheet).
    OneMore(Viewport),
}

/// The timer, held by the practice log: what the store keeps of it, the clock's last tick, the
/// block in hand and its break (S124b fills the state it needs).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Pomodoro {
    /// The settings, the block under way and the blocks completed, as last kept.
    kept: Blocks,
    /// The clock's last tick, seconds since 1970 UTC.
    now: u64,
}

impl Pomodoro {
    /// The timer as the store kept it.
    pub(crate) fn of(kept: Blocks) -> Self {
        // S124b: a block under way read back held, as a restart leaves it.
        Self { kept, now: 0 }
    }

    /// The blocks completed, oldest first: the book's and the home's seals.
    pub(crate) fn done(&self) -> &[Block] {
        &self.kept.done
    }

    /// The clock's last tick.
    pub(crate) const fn now(&self) -> u64 {
        self.now
    }

    /// The clock at `now`, today being local day `today`, a day's or a test's sheet in front
    /// (`front`) or not.
    pub(crate) fn tick(&mut self, now: u64, (today, front): (i64, bool)) {
        // S124b: count down, hold, drop a held block of a past day, the break and its chime.
        let _ = (today, front);
        self.now = now;
    }

    /// The settings as kept.
    pub(crate) const fn timing(&self) -> openpaint_practice::Timing {
        self.kept.timing
    }
}
