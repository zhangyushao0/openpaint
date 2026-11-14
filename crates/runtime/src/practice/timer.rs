//! The Pomodoro in the runtime (S124; its flow is the practice log's, `PracticeLog::tick` and
//! beside it): each frame the wall clock ticks the timer with the sheet in front before the UI
//! pass (`Runtime::clocked`), the app leaving ticks it with none (`host.rs`), its intents apply
//! after the sheet's and the options' (`timed`), and what it changed goes to the store. The host
//! asks after each frame when the timer wants the next (`Runtime::wake_in`: Android posts a frame
//! then; nothing draws at rest otherwise), whether the screen stays on (`Runtime::screen_on`:
//! Android's keep-screen-on flag; nothing elsewhere) and whether the break's chime sounds
//! (`Runtime::take_chime`: Android plays a short system tone, no asset; nothing elsewhere). A
//! scenario moves the held clock with its `later` step (`{ "later": { "minutes": 25 } }`: the
//! app in front untouched, one frame), `hidden` for the app left meanwhile.

use std::time::Duration;

use openpaint_app::{Gallery, GalleryIntent};
use openpaint_practice_app::{
    Options, OptionsIntent, PracticeLog, PracticeSheet, Start, Timed, TimerIntent,
};

use super::jobs::PracticeJob;
use crate::canvas::Canvas;
use crate::frame::Runtime;
use crate::seeds::Seeds;
use crate::storage::{Job, Storage};

impl Runtime {
    /// The frame's clock, `now` seconds since 1970 UTC, to the practice log: the local day, and
    /// the timer's tick with the sheet in front; what the timer changed to the store.
    pub(crate) fn clocked(&mut self, now: u64) {
        self.practice.set_clock(now, self.utc_offset);
        let front = (self.screen.canvas())
            .and_then(|canvas| PracticeSheet::of(canvas.session()))
            .map(PracticeSheet::record);
        self.practice.tick(now, front);
        timer_saved(&mut self.practice, &mut self.storage);
    }

    /// How long after the last frame the timer wants one (the turn of the minute it shows, the
    /// end of the block or of its break); none while nothing counts. The host asks after each
    /// frame and asks for a frame then.
    #[must_use]
    pub fn wake_in(&self) -> Option<Duration> {
        self.practice.wake().map(Duration::from_secs)
    }

    /// Whether the screen stays on: a Pomodoro's block or its break counts. The host keeps it on
    /// while this holds (Android's keep-screen-on flag on its window).
    #[must_use]
    pub fn screen_on(&self) -> bool {
        self.practice.screen_on()
    }

    /// Whether the break's chime sounds now, once (the practice options' switch on). The host
    /// plays one soft system tone.
    pub fn take_chime(&mut self) -> bool {
        self.practice.take_chime()
    }
}

/// The timer's records to the store, once after each change.
pub(crate) fn timer_saved(log: &mut PracticeLog, storage: &mut Storage) {
    if let Some(write) = log.timer_request() {
        storage.submit(Job::Practice(PracticeJob::Timer(write)));
    }
}

/// The frame's timer intents on the open sheet, after its own: Another pomodoro lays the next
/// lesson as One more does (`super::lay_next`).
pub(crate) fn timed_on_sheet(
    canvas: &mut Canvas,
    intents: Vec<TimerIntent>,
    (log, seeds, storage): (&mut PracticeLog, &mut Seeds, &mut Storage),
    now: u64,
) {
    for intent in intents {
        match log.time(intent, Some(canvas.session_mut())) {
            Ok(Timed::OneMore(viewport)) => {
                super::lay_next(
                    canvas,
                    (&mut *log, &mut *seeds, &mut *storage),
                    (now, &viewport),
                );
            }
            Ok(Timed::Done) => {}
            Ok(Timed::Lesson) => log::warn!("a lesson asked of the timer on a sheet"),
            Err(error) => log::warn!("timer intent refused: {error}"),
        }
    }
}

/// The frame's timer intents on the home, after the options': Start timer (or Continue) opens the
/// lesson under way, else starts the one Start starts.
pub(crate) fn timed_at_home(
    (gallery, options): (&mut Gallery, &mut Options),
    intents: Vec<TimerIntent>,
    log: &mut PracticeLog,
) {
    for intent in intents {
        let opened = match log.time(intent, None) {
            Ok(Timed::Lesson) => match log.path().open {
                Some(open) => gallery.dispatch(GalleryIntent::Open(open.sheet)),
                None => options.dispatch(OptionsIntent::Lesson(Start::Next), gallery),
            },
            Ok(Timed::Done) => Ok(()),
            Ok(Timed::OneMore(_)) => {
                log::warn!("One more asked of the timer on the home");
                Ok(())
            }
            Err(error) => {
                log::warn!("timer intent refused: {error}");
                Ok(())
            }
        };
        if let Err(error) = opened {
            log::warn!("the timer's lesson did not open: {error}");
        }
    }
}
