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
//!   sheet opening), the app hidden (`PracticeLog::hidden`, the runtime's `leave`) or the bar's
//!   Pause. It counts on when the sheet is in front again (and Continue, if the user paused it).
//!   A held block whose local day is past at a tick is dropped: nothing kept, no seal.
//! - Time up (no second left): the ring red and Time in the bar; nothing interrupts the stroke;
//!   the exercise in hand may be finished and compared; Next reads End (`TimerIntent::End`). The
//!   projection a frame's UI pass reads is that frame's tick's, so a Next tapped as the time runs
//!   out is laid out as End: the block ends there.
//! - The chain: while a block counts with time left, a lesson's last exercise passed (Next) ends
//!   the lesson as ever, and the runtime at once lays the next lesson as One more would
//!   (`PracticeLog::chains`; `End::next`: the path's next after this lesson counts, or the same
//!   skill again after a lesson of a unit past the learner's) on the next row of the day's sheet,
//!   the view gliding to its first pair, or on a new sheet when it does not fit. No end card shows
//!   between (the screens hide a lesson's end while a block runs), unless the store refused the
//!   lesson laid: rolled back, the end shows with its One more. A kind's How still comes once.
//! - End with no exercise of the block begun (drawn in or compared; a lesson the chain left on
//!   another sheet counts as begun): as Stop timer, no seal and no end, the lesson in hand as it
//!   was (`PracticeLog::begun_in`).
//! - End: the lesson in hand ends with the exercises begun (the one in hand stays; those after it
//!   are taken off the sheet, its record and its stored givens: `PracticeSheet::trim_request`,
//!   `practice_store::trim`), finished at the tick; the view fits the rows drawn in the block; the
//!   block is completed (`TimerWrite::completed`: its seal); its end shows (`BlockEnd`) and the
//!   break of five minutes (`BREAK`) counts while the sheet is in front.
//! - The break: its ring counts down with the minutes left; at its end one soft chime (when the
//!   switch is on, `Timing::chime`; `PracticeLog::take_chime`, once) and the ring complete,
//!   nothing else. Another pomodoro works at once, during the break or after; That's all for today
//!   (or Home) leaves the block's end, which ends the break silently; the app hidden holds the
//!   break, the end kept for its return. The break is never stored.
//! - Stop timer (under way or time up): the block goes, nothing kept, no seal; the lesson in hand
//!   goes on untimed, as any lesson.
//!
//! Frames: the app draws none at rest; the timer wants one at the turn of each minute shown and
//! at the end of the block or the break (`PracticeLog::wake`), never every second. The screen
//! stays on while the block or the break counts (`PracticeLog::screen_on`). What changed goes to
//! the store at each transition and once a minute while it counts (`PracticeLog::timer_request`).

use std::sync::Arc;

use openpaint_app::{Availability, Viewport};
use openpaint_practice::{BREAK, Block, BlockTally, Blocks, Length, Sheet, TimerWrite, UnderWay};

use crate::refused::Refused;

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
/// block just completed with its break, and what the store and the host have not had yet.
///
/// The seconds between two ticks belong to the state the first of them left: they count only if
/// a day's or a test's sheet was in front at it (and the user had not paused), so a tick that
/// comes late, a clock that jumps and a clock that stands still all count exactly what was in
/// front.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Pomodoro {
    /// The settings, the block under way and the blocks completed, as last kept.
    kept: Blocks,
    /// The clock's last tick, seconds since 1970 UTC (0: none yet since the app started), the
    /// local day then, and whether a day's or a test's sheet was in front then.
    now: u64,
    today: i64,
    front: bool,
    /// The seconds the block under way began with (its length; a test chore's fewer).
    began: u32,
    /// The last local day the block under way ran on (in front, not paused).
    ran_on: i64,
    /// The block just completed, while its end is in front: its card and its break.
    ended: Option<Ended>,
    /// The break's chime is due, taken once.
    chime: bool,
    /// Not yet written: a change since the last write, a block completed; the seconds left the
    /// last write held (once a minute while it counts).
    unsaved: bool,
    completed: Option<Block>,
    saved_left: u32,
}

/// A block completed, its end in front: the block, its card and the break's seconds left.
#[derive(Debug, Clone, PartialEq)]
struct Ended {
    block: Block,
    end: Arc<BlockEnd>,
    rest: u32,
}

impl Pomodoro {
    /// The timer as the store kept it: a block under way comes back held (the seconds until the
    /// first tick are not counted), its seconds left and its pause as kept.
    pub(crate) fn of(kept: Blocks) -> Self {
        let block = kept.under_way;
        Self {
            began: block.map_or(0, |block| block.length.seconds()),
            ran_on: block.map_or(0, |block| block.day),
            saved_left: block.map_or(0, |block| block.left),
            kept,
            ..Self::default()
        }
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
    /// (`front`) or not: the seconds since the last tick counted down where they count, a held
    /// block of a past day dropped, the break counted and its end left once nothing is in front.
    pub(crate) fn tick(&mut self, now: u64, (today, front): (i64, bool)) {
        let since = if self.now == 0 {
            0
        } else {
            now.saturating_sub(self.now)
        };
        let seconds = u32::try_from(since).unwrap_or(u32::MAX);
        let was = self.front;
        // A clock set back counts nothing until it passes the last tick again.
        (self.now, self.today, self.front) = (self.now.max(now), today, front);
        self.counted(seconds, was);
        let ran = was && front;
        if let Some(block) = self.kept.under_way {
            if ran && !block.paused {
                self.ran_on = today;
            } else if today > self.ran_on {
                // Held when the day changed: nothing kept, no seal.
                self.kept.under_way = None;
                self.unsaved = true;
            }
        }
        self.rested(seconds, was, front);
    }

    /// The app hidden at `now`: what counted until then counted, and from then on nothing counts,
    /// the block and its break held alike; the block's end stays for when the sheet is back.
    pub(crate) fn hidden(&mut self, now: u64) {
        self.tick(now, (self.today, self.front));
        self.front = false;
    }

    /// `seconds` off the block under way, if it counted since the last tick (`front` then).
    fn counted(&mut self, seconds: u32, front: bool) {
        let Some(block) =
            (self.kept.under_way.as_mut()).filter(|block| front && !block.paused && block.left > 0)
        else {
            return;
        };
        block.left = block.left.saturating_sub(seconds);
        if block.left == 0 || self.saved_left.saturating_sub(block.left) >= 60 {
            self.unsaved = true;
        }
    }

    /// `seconds` off the break, if its end was in front (`was`), the chime due at its end; the
    /// end left (nothing in front `now`) ends it silently.
    fn rested(&mut self, seconds: u32, was: bool, now: bool) {
        let Some(ended) = self.ended.as_mut() else {
            return;
        };
        if was && ended.rest > 0 {
            ended.rest = ended.rest.saturating_sub(seconds);
            self.chime = ended.rest == 0 && self.kept.timing.chime;
        }
        if !now {
            (self.ended, self.chime) = (None, false);
        }
    }

    /// The length the home's row rings, kept for the next block.
    pub(crate) fn choose(&mut self, length: Length) {
        self.kept.timing.length = length;
        self.unsaved = true;
    }

    /// The practice options' switch: the break's end chimes or not.
    pub(crate) fn chime_on(&mut self, on: bool) {
        self.kept.timing.chime = on;
        self.unsaved = true;
    }

    /// A block of the length chosen begins at the last tick, unless one is under way; the end of
    /// the one before and its break go.
    pub(crate) fn begin(&mut self) {
        if self.kept.under_way.is_some() {
            return;
        }
        let length = self.kept.timing.length;
        self.kept.under_way = Some(UnderWay {
            started: self.now,
            day: self.today,
            length,
            left: length.seconds(),
            paused: false,
        });
        (self.ran_on, self.saved_left, self.began) =
            (self.today, length.seconds(), length.seconds());
        (self.ended, self.chime, self.unsaved) = (None, false, true);
    }

    /// For debug builds' test chore only: a block begun as `begin` begins one, only `seconds`
    /// left of it, unless one is under way; whether it was.
    pub(crate) fn begin_test(&mut self, seconds: u32) -> bool {
        if self.kept.under_way.is_some() {
            return false;
        }
        self.begin();
        if let Some(block) = self.kept.under_way.as_mut() {
            block.left = block.left.min(seconds.max(1));
            (self.saved_left, self.began) = (block.left, block.left);
        }
        true
    }

    /// For debug builds' test chore only: the blocks begun at or after `since` gone, completed
    /// or under way, with the end of one in front.
    pub(crate) fn forget(&mut self, since: u64) {
        self.kept.done.retain(|block| block.started < since);
        self.completed = self.completed.filter(|block| block.started < since);
        if self
            .kept
            .under_way
            .is_some_and(|block| block.started >= since)
        {
            self.kept.under_way = None;
            self.unsaved = true;
        }
        if (self.ended.as_ref()).is_some_and(|ended| ended.block.started >= since) {
            (self.ended, self.chime) = (None, false);
        }
    }

    /// The bar's Pause (`paused`) or Continue, while seconds are left.
    pub(crate) fn pause(&mut self, paused: bool) -> Result<(), Refused> {
        let block = (self.kept.under_way.as_mut()).filter(|block| block.left > 0);
        let block = block.ok_or(Refused::Unavailable(Availability::NothingLeft))?;
        block.paused = paused;
        self.unsaved = true;
        Ok(())
    }

    /// Stop timer: the block under way goes, nothing kept.
    pub(crate) fn stop(&mut self) -> Result<(), Refused> {
        self.kept
            .under_way
            .take()
            .ok_or(Refused::Unavailable(Availability::NothingLeft))?;
        self.unsaved = true;
        Ok(())
    }

    /// The block under way, once its time is up.
    pub(crate) fn up(&self) -> Option<UnderWay> {
        self.kept.under_way.filter(|block| block.left == 0)
    }

    /// Whether a block is under way with time left: a lesson ended then chains the next.
    pub(crate) fn counts_on(&self) -> bool {
        self.kept.under_way.is_some_and(|block| block.left > 0)
    }

    /// Whether a block's end is in front (Another pomodoro takes).
    pub(crate) const fn ended(&self) -> bool {
        self.ended.is_some()
    }

    /// The block under way completed at End: kept for its seal, written once, its end shown (its
    /// tally made by `tallied` as the log takes the sheet End cut) and its break begun.
    pub(crate) fn complete(&mut self) {
        let Some(block) = self.kept.under_way.take() else {
            return;
        };
        let block = Block {
            started: block.started,
            day: block.day,
            length: block.length,
        };
        self.kept.done.push(block);
        self.completed = Some(block);
        self.unsaved = true;
        let end = BlockEnd {
            seconds: self.began,
            tally: BlockTally::default(),
        };
        self.ended = Some(Ended {
            block,
            end: Arc::new(end),
            rest: BREAK,
        });
    }

    /// The block's end counted again from `sheets` (as the log changes, never on a frame).
    pub(crate) fn tallied(&mut self, sheets: &[Sheet]) {
        if let Some(ended) = self.ended.as_mut() {
            let tally = BlockTally::of(sheets, ended.block.started);
            if tally != ended.end.tally {
                ended.end = Arc::new(BlockEnd {
                    seconds: ended.end.seconds,
                    tally,
                });
            }
        }
    }

    /// Whether the break's chime sounds now: once.
    pub(crate) fn take_chime(&mut self) -> bool {
        std::mem::take(&mut self.chime)
    }

    /// The timer's records for the store, once after each change and once a minute while a
    /// block counts.
    pub(crate) fn request(&mut self) -> Option<TimerWrite> {
        if !std::mem::take(&mut self.unsaved) {
            return None;
        }
        let under_way = self.kept.under_way;
        self.saved_left = under_way.map_or(0, |block| block.left);
        Some(TimerWrite {
            timing: self.kept.timing,
            under_way,
            completed: self.completed.take(),
        })
    }
}
