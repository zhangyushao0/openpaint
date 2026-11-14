//! The Pomodoro's records (S124): a block is a timed stretch of lessons one after another, 15,
//! 25 or 40 minutes of the sheet in front (`Length`), the length chosen and the break's chime kept
//! as the timer's settings (`Timing`); the block under way kept so it survives the sheet left and
//! a restart (`UnderWay`: its seconds left, whether the user paused it); a block completed kept
//! for its seal (`Block`: its local day, its start, its length). What the store keeps of them all
//! is `Blocks`, one write of them `TimerWrite`; their bytes are practice-store's (`blocks.rs`).
//!
//! Rules every part keeps:
//! - Only a completed block is a record of its own. A block stopped (Stop timer), or under way
//!   and found not running on a later local day than its own, leaves nothing: no block, no seal,
//!   only the lessons drawn in it, which are the sheets' as any lesson is.
//! - A block's seal counts on the local day it started (`Block::day`), even when it ran past
//!   midnight.
//! - The break after a block (`BREAK`) is never stored: a restart, or the block's end left, ends
//!   it without a chime.
//! - What a block's end shows is counted from the sheets, never stored (`BlockTally`): its
//!   lessons are those finished at or after its start (`UnderWay::started`) on a day's or a
//!   test's sheet; a lesson continued into a block counts whole.
//!
//! No streak, no daily goal: blocks count what was done, never days in a row (D109).

use crate::kinds::Skill;
use crate::marks::compared::Outcome;
use crate::records::sheet::Sheet;

/// A break's length after a block completed, in seconds: five minutes.
pub const BREAK: u32 = 300;

/// A block's length as the home's row offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Length {
    Fifteen,
    /// The length chosen at first.
    #[default]
    TwentyFive,
    Forty,
}

impl Length {
    /// The three, shortest first, as the home's row lays them.
    pub const ALL: [Self; 3] = [Self::Fifteen, Self::TwentyFive, Self::Forty];

    /// Its minutes.
    #[must_use]
    pub const fn minutes(self) -> u8 {
        match self {
            Self::Fifteen => 15,
            Self::TwentyFive => 25,
            Self::Forty => 40,
        }
    }

    /// Its seconds.
    #[must_use]
    pub const fn seconds(self) -> u32 {
        self.minutes() as u32 * 60
    }

    /// Its stored code: its minutes (15, 25, 40).
    #[must_use]
    pub const fn code(self) -> u8 {
        self.minutes()
    }

    /// The length stored as `code`; none for another code (refused).
    #[must_use]
    pub const fn of_code(code: u8) -> Option<Self> {
        match code {
            15 => Some(Self::Fifteen),
            25 => Some(Self::TwentyFive),
            40 => Some(Self::Forty),
            _ => None,
        }
    }
}

/// The timer's settings, kept: the length the home's row has ringed, and whether a break's end
/// sounds its chime (the practice options' switch, on at first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    pub length: Length,
    pub chime: bool,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            length: Length::default(),
            chime: true,
        }
    }
}

/// The block under way, as kept between its transitions (begun, paused, resumed, time up, and
/// once a minute while it runs): when it started (seconds since 1970 UTC) and on which local day
/// (`time::local_day`), its length, the seconds left (0: the time is up, End waits) and whether
/// the user paused it (the bar's Pause; a sheet not in front pauses it without this).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderWay {
    pub started: u64,
    pub day: i64,
    pub length: Length,
    pub left: u32,
    pub paused: bool,
}

/// A block completed: its seal on the home's Practice header (today's) and on its day in the
/// practice book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// When it started, seconds since 1970 UTC; its key among the blocks.
    pub started: u64,
    /// The local day it started on.
    pub day: i64,
    pub length: Length,
}

/// What the store keeps of the Pomodoro: the settings, the block under way and every block
/// completed, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Blocks {
    pub timing: Timing,
    pub under_way: Option<UnderWay>,
    pub done: Vec<Block>,
}

/// One write of the timer's records, whole or not at all: the settings and the block under way
/// as they stand (none: no block under way), and a block completed since the last write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerWrite {
    pub timing: Timing,
    pub under_way: Option<UnderWay>,
    pub completed: Option<Block>,
}

impl Blocks {
    /// How many blocks were completed on local day `day`: the red seals of that day.
    #[must_use]
    pub fn on(&self, day: i64) -> usize {
        // S124a
        let _ = day;
        0
    }
}

/// What a block's end shows, counted from the sheets (S124): its lessons and exercises, each
/// exercise's result in order (none: not judged; the first lesson's never scored, so none), and
/// each skill that gained a dot in it with its dots before the block and after it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockTally {
    pub lessons: u32,
    pub exercises: u32,
    pub results: Vec<Option<Outcome>>,
    pub skills: Vec<(Skill, u8, u8)>,
}

impl BlockTally {
    /// What the lessons of `sheets` finished at or after `since` (a block's start) on a day's or
    /// a test's sheet did, in the order of sheet and lesson (`lessons`); the learner before them
    /// is the one every other lesson tells of (`Learner::of` without them), after them the one all
    /// tell of.
    #[must_use]
    pub fn of(sheets: &[Sheet], since: u64) -> Self {
        // S124a
        let _ = (sheets, since);
        Self::default()
    }
}
