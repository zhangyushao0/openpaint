//! What the timer tells the app's log (S124): each of its transitions once, as it happens, for
//! the runtime to write at info (`PracticeLog::timer_told`), so a run on the tablet reads as the
//! user's walk. Nothing here decides anything.

use super::Pomodoro;

/// One transition of the timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerTold {
    /// A block began with this many seconds (its length; a test chore's fewer).
    Started(u32),
    /// The block counts: a day's or a test's sheet in front, not paused.
    Counts,
    /// The block is held: no such sheet in front, the app hidden, or paused.
    Held,
    /// The time is up.
    Up,
    /// End completed the block: the lessons and exercises its end counts.
    Completed { lessons: u32, exercises: u32 },
    /// End with nothing of the block begun: left as Stop timer leaves it.
    Stopped,
    /// The break began with this many seconds.
    BreakStarted(u32),
    /// The break counted to its end; its chime is due (the switch on) or not (off).
    BreakEnded { chime: bool },
    /// The block's end left before its break was over: the break ends with no chime.
    BreakLeft,
}

impl Pomodoro {
    /// After any change: whether the block counts now, told when it changed while time is left.
    pub(super) fn note(&mut self) {
        let left = self.kept.under_way.filter(|block| block.left > 0);
        let counts = self.front && left.is_some_and(|block| !block.paused);
        if counts != self.counted {
            self.counted = counts;
            if left.is_some() {
                self.told.push(if counts {
                    TimerTold::Counts
                } else {
                    TimerTold::Held
                });
            }
        }
    }

    /// End's outcome: the block completed with its end counted, and its break begun; else
    /// nothing begun.
    pub(crate) fn tell_end(&mut self) {
        let Some(ended) = &self.ended else {
            self.told.push(TimerTold::Stopped);
            return;
        };
        let tally = &ended.end.tally;
        let completed = TimerTold::Completed {
            lessons: tally.lessons,
            exercises: tally.exercises,
        };
        let rest = TimerTold::BreakStarted(ended.rest);
        self.told.extend([completed, rest]);
    }

    /// The transitions since the last call, oldest first.
    pub(crate) fn told(&mut self) -> Vec<TimerTold> {
        std::mem::take(&mut self.told)
    }
}
