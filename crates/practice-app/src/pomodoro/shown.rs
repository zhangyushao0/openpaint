//! The Pomodoro as one UI frame shows it (S124; `PracticeLog::timer`, made from the frame's tick):
//! the home's row (its length, Start timer, or a block under way with Continue), the red seals of
//! today at the right of the Practice header, the bar's ring before the lesson's place, and a
//! block's end over the rows drawn in it. The screens read it and never assemble a rule of their
//! own (when Next reads End, whether a lesson's end shows, how many seals) from anything else.

use std::sync::Arc;

use openpaint_practice::{BREAK, BlockTally, Length};

use super::Pomodoro;

/// The timer for one UI frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimerShown {
    /// The length the home's row has ringed: the next block's.
    pub length: Length,
    /// Whether a break's end chimes: the practice options' switch.
    pub chime: bool,
    /// Today's blocks completed: the small red seals at the right of the Practice header, shown
    /// only when there is one; the book's days hold their own (`Day::blocks`).
    pub sealed: u8,
    /// The block in hand; none while the home's row offers Start timer.
    pub block: Option<BlockShown>,
}

/// The block in hand.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockShown {
    /// Under way: the minutes left, rounded up ("18 min", "18 分"), and the share of its length
    /// left (the ring's arc); `paused` by the user (the ring's menu offers Continue, else Pause).
    /// On the home: the ring, the minutes left and Continue in place of Start timer. While it
    /// shows, a lesson's end (`PracticeProjection::end`) does not: the chain lays the next (unless
    /// the store refused it, `PracticeProjection::rolled_back`).
    Running { left: u32, share: f32, paused: bool },
    /// The time is up: the ring red and Time (到点) where the minutes were, no menu; Next reads
    /// End (`TimerIntent::End`). On the home: the red ring, Time and Continue.
    Up,
    /// Its end, on the sheet in place of a lesson's end: the card (`BlockEnd`), and the break's
    /// minutes left with the share of it left while it counts (none once it is over: the ring
    /// complete). Another pomodoro and That's all for today.
    Ended {
        end: Arc<BlockEnd>,
        rest: Option<(u32, f32)>,
    },
}

/// A block's end as its card shows it: its length ("25 min"), "5 lessons · 19 exercises", each
/// result as a mark, each skill that gained a dot with its dots, the seal (a red seal reading 番茄
/// and its English word).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockEnd {
    pub length: Length,
    pub tally: BlockTally,
}

impl Pomodoro {
    /// The timer as the frame shows it at the last tick.
    pub(crate) fn shown(&self) -> TimerShown {
        let timing = self.kept.timing;
        let block = match (&self.ended, self.kept.under_way) {
            (Some(ended), _) => Some(BlockShown::Ended {
                end: Arc::clone(&ended.end),
                rest: (ended.rest > 0).then(|| (minutes(ended.rest), share(ended.rest, BREAK))),
            }),
            (None, Some(block)) if block.left == 0 => Some(BlockShown::Up),
            (None, Some(block)) => Some(BlockShown::Running {
                left: minutes(block.left),
                share: share(block.left, block.length.seconds()),
                paused: block.paused,
            }),
            (None, None) => None,
        };
        TimerShown {
            length: timing.length,
            chime: timing.chime,
            sealed: u8::try_from(self.kept.on(self.today)).unwrap_or(u8::MAX),
            block,
        }
    }

    /// The seconds counting down now: the block's (in front, not paused) or its break's (its end
    /// in front); none while nothing counts.
    fn counting(&self) -> Option<u32> {
        if !self.front {
            return None;
        }
        match (&self.ended, self.kept.under_way) {
            (Some(ended), _) => Some(ended.rest),
            (None, Some(block)) if !block.paused => Some(block.left),
            _ => None,
        }
        .filter(|left| *left > 0)
    }

    /// Seconds after the last tick at which the minute shown turns, or the block or its break
    /// ends (the last minute's turn); none while nothing counts.
    pub(crate) fn wake(&self) -> Option<u64> {
        let left = self.counting()?;
        Some(u64::from(left - 60 * (minutes(left) - 1)))
    }

    /// Whether the screen stays on: the block or its break counts.
    pub(crate) fn screen_on(&self) -> bool {
        self.counting().is_some()
    }
}

/// `seconds` as the minutes shown: rounded up.
const fn minutes(seconds: u32) -> u32 {
    seconds.div_ceil(60)
}

/// The share `left` is of `all`.
fn share(left: u32, all: u32) -> f32 {
    (left as f32 / all.max(1) as f32).clamp(0.0, 1.0)
}
