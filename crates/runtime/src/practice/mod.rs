//! Practice composed into the runtime (G9, D112): the painting crates offer a session a mode sets
//! and steps and the gallery's turn for pickers, and this module puts practice into them. A sheet
//! opens on the canvas owning its session (`Canvas::open`, `SheetSession`); its intents reach it
//! after the frame's painting intents (`Canvas::practise`), One more laying the next lesson on the
//! same sheet or leaving it for a new one (`apply`, `day.rs`, the day's sheet); what the sheet
//! asks of the store and the workers, and what they answer, passes through `sheet`, the one place
//! the runtime reaches the sheet's flow; the practice options take their turn beside the gallery
//! (`options.rs`); practice's part of the screen (`Shown`) fills the shell's hooks through its
//! screens (`openpaint_practice_ui::PracticeScreens`). The store's jobs are `jobs.rs` and
//! `files.rs`, Compare's worker `compare.rs`, the options' pictures `pack_pictures.rs`, the test
//! lessons of debug builds `test.rs`. Every lesson of the path (S122) starts in `lesson.rs`. The
//! Pomodoro's clock, intents and host calls are `timer.rs` (S124); its chain lays the next lesson
//! here as One more does (`apply`, `lay_next`).

mod compare;
mod day;
mod files;
mod jobs;
mod lesson;
mod options;
mod pack_pictures;
mod test;
mod timer;

use openpaint_app::Viewport;
use openpaint_practice::Purpose;
use openpaint_practice_app::{
    OptionsNotice, PathProjection, PracticeIntent, PracticeLog, PracticeProjection, PracticeSheet,
    SheetSession, Start, TimerShown,
};
use openpaint_practice_ui::{Hooked, PracticeScreens};

use crate::canvas::{Canvas, Leaving, Then};
use crate::screen::Screen;
use crate::seeds::Seeds;
use crate::storage::{Job, Storage};

pub(crate) use compare::CompareWorker;
pub(crate) use day::{appended, one_more};
pub(crate) use jobs::{
    PracticeDone, PracticeJob, copy_job, gallery_list, run, seed, sheet_printed, stopped,
};
pub(crate) use lesson::lesson_job;
pub(crate) use options::{chosen, dispatch_options, submit_options};

use crate::frame::Runtime;
pub(crate) use pack_pictures::PackPictureWorker;
pub use test::{PracticeTest, skill_start};
pub(crate) use timer::{timed_at_home, timed_on_sheet, timer_saved};

/// The practice sheet open on `canvas`, if one is: the one place the runtime reaches its flow
/// (`SheetSession::sheet_mut`).
pub(crate) fn sheet(canvas: &mut Canvas) -> Option<&mut PracticeSheet> {
    canvas.sheet_mut().map(SheetSession::sheet_mut)
}

/// The open sheet's record to the store and into `log`, once it changed since it last went (its
/// time with it); a record the Pomodoro's End cut goes first, with its givens (S124). The kinds
/// whose How showed while it was used go into `log` too, so a sheet opened again never tells
/// them again; so do the blind pairs its end asked and those answered (S147).
pub(crate) fn save_record(canvas: &mut Canvas, (log, storage): (&mut PracticeLog, &mut Storage)) {
    if let Some(sheet) = sheet(canvas) {
        log.tell(sheet.seen());
        // S147: the pairs its end asked (none asked again today), each answer stored once, the
        // copies' cards read.
        log.asked(sheet.offered());
        for pair in sheet.pairs_request() {
            log.keep_pair(pair);
            storage.submit(Job::Practice(PracticeJob::Pair(pair)));
        }
    }
    if let Some(copies) = canvas.sheet_mut().and_then(SheetSession::copies_request) {
        let job = PracticeJob::Cards(log.copies_of(&copies), log.offset());
        storage.submit(Job::Practice(job));
    }
    if let Some(record) = sheet(canvas).and_then(PracticeSheet::trim_request) {
        log.keep(record.clone());
        storage.submit(Job::Practice(PracticeJob::Trim(record)));
    }
    if let Some(record) = sheet(canvas).and_then(PracticeSheet::save_request) {
        log.keep(record.clone());
        storage.submit(Job::Practice(PracticeJob::Save(record)));
    }
}

/// The frame's sheet intents at `now` (seconds since 1970 UTC), after its painting intents, each
/// applied as one of them is (`Canvas::practise`); One more, and a Next that ended a lesson while
/// a Pomodoro's block counts with time left (the chain, `PracticeLog::chains`), lay the next
/// lesson (`lay_next`).
pub(crate) fn apply(
    canvas: &mut Canvas,
    intents: Vec<PracticeIntent>,
    (log, seeds, storage): (&mut PracticeLog, &mut Seeds, &mut Storage),
    now: u64,
) {
    for intent in intents {
        let applied = canvas.practise(|sheet| sheet.practise(intent, now));
        let next = match intent {
            PracticeIntent::OneMore(viewport) | PracticeIntent::Drill(viewport) => {
                applied.then_some(viewport)
            }
            PracticeIntent::Next(viewport) => {
                let chains = canvas.sheet().is_some_and(|open| log.chains(open.sheet()));
                (applied && chains).then_some(viewport)
            }
            _ => None,
        };
        if let Some(viewport) = next {
            // An extra drill (S147) lays its skill's lesson as One more lays the next.
            let start = match intent {
                PracticeIntent::Drill(_) => day::drill(canvas),
                _ => day::named(canvas),
            };
            lay_next(
                canvas,
                (&mut *log, &mut *seeds, &mut *storage),
                (start, now, &viewport),
            );
        }
    }
}

/// `start`'s lesson after the one `canvas`'s sheet just ended (One more's: `End::next`,
/// `Start::Next` where it names none; an extra drill's) on the same sheet where it takes it
/// (`day.rs`, the lesson begun from `log` seeded from `seeds`, written by `storage`), the view
/// gliding to it in `viewport`; else the sheet left for a new one.
pub(crate) fn lay_next(
    canvas: &mut Canvas,
    app: (&mut PracticeLog, &mut Seeds, &mut Storage),
    (start, now, viewport): (Start, u64, &Viewport),
) {
    if !day::one_more(canvas, app, (start, now, viewport)) {
        // A test's sheet left for a new one stays a test's (`practice-test`, debug builds).
        let purpose = (sheet(canvas).map(|sheet| sheet.record().purpose))
            .filter(|purpose| *purpose == Purpose::Test)
            .unwrap_or(Purpose::Day);
        canvas.leaving = Leaving::Waiting;
        canvas.then = Then::Practice(start, purpose);
    }
}

/// Practice's part of the screen for one frame: the open sheet's, or on the home the path and
/// the options' notice; the Pomodoro's timer on either (S124).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Shown {
    sheet: Option<PracticeProjection>,
    path: Option<PathProjection>,
    notice: Option<OptionsNotice>,
    timer: TimerShown,
}

impl Shown {
    /// What `screen` shows of practice, the path and the timer from `log`.
    pub(crate) fn of(screen: &Screen, log: &PracticeLog) -> Self {
        let timer = log.timer();
        match screen {
            Screen::Canvas(canvas) => Self {
                sheet: canvas.sheet().map(SheetSession::projection),
                timer,
                ..Self::default()
            },
            Screen::Gallery(_, options) => Self {
                path: Some(log.path()),
                notice: options.notice(),
                timer,
                ..Self::default()
            },
            Screen::Opening(_) => Self::default(),
        }
    }

    /// The hooks `screens` fill for it in one frame.
    pub(crate) fn hooked<'a>(&'a self, screens: &'a mut PracticeScreens) -> Hooked<'a> {
        let path = self.path.as_ref().map(|path| (path, self.notice));
        screens.hooked((self.sheet.as_ref(), path, &self.timer))
    }
}

impl Runtime {
    /// The open practice sheet, while the canvas shows one.
    #[must_use]
    pub fn practice(&self) -> Option<PracticeProjection> {
        self.screen.canvas()?.sheet().map(SheetSession::projection)
    }

    /// The path (S122: the home's practice column, the path page, the practice book), while the
    /// home shows.
    #[must_use]
    pub fn path(&self) -> Option<PathProjection> {
        matches!(self.screen, Screen::Gallery(..)).then(|| self.practice.path())
    }
}
