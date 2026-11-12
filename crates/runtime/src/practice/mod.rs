//! Practice composed into the runtime (G9, D112): the painting crates know only a mode slot on the
//! session and the gallery's turn for pickers, and this module puts practice into them. A sheet
//! opens as the session's mode (`Canvas::open`); its intents reach it after the frame's painting
//! intents, One more laying the next lesson on the same sheet or leaving it for a new one (`apply`,
//! `day.rs`, the day's sheet); what the sheet asks of
//! the store and the workers, and what they answer, passes through `sheet`, the one place the
//! runtime reaches the sheet; the practice options take their turn beside the gallery
//! (`options.rs`); practice's part of the screen (`Shown`) fills the shell's hooks through its
//! screens (`openpaint_practice_ui::PracticeScreens`). The store's jobs are `jobs.rs` and
//! `files.rs`, Compare's worker `compare.rs`, the options' pictures `pack_pictures.rs`, the test
//! lessons of debug builds `test.rs`. Every lesson of the path (S122) starts in `lesson.rs`.

mod compare;
mod day;
mod files;
mod jobs;
mod lesson;
mod options;
mod pack_pictures;
mod test;

use openpaint_practice_app::{
    OptionsNotice, PathProjection, PracticeIntent, PracticeLog, PracticeProjection, PracticeSheet,
    practise,
};
use openpaint_practice_ui::{Hooked, PracticeScreens};

use crate::canvas::{Canvas, Leaving, Then};
use crate::screen::Screen;
use crate::seeds::Seeds;
use crate::storage::{Job, Storage};

pub(crate) use compare::CompareWorker;
pub(crate) use day::appended;
pub(crate) use jobs::{
    PracticeDone, PracticeJob, copy_job, gallery_list, run, seed, sheet_printed, stopped,
};
pub(crate) use lesson::lesson_job;
pub(crate) use options::{chosen, dispatch_options, submit_options};

use crate::frame::Runtime;
pub(crate) use pack_pictures::PackPictureWorker;
pub use test::{PracticeTest, skill_start};

/// The practice sheet `canvas`'s session holds, if it holds one: the one place the runtime
/// reaches it.
pub(crate) fn sheet(canvas: &mut Canvas) -> Option<&mut PracticeSheet> {
    PracticeSheet::of_mut(canvas.session_mut())
}

/// The open sheet's record to the store and into `log`, once it changed since it last went (its
/// time with it).
pub(crate) fn save_record(canvas: &mut Canvas, (log, storage): (&mut PracticeLog, &mut Storage)) {
    if let Some(record) = sheet(canvas).and_then(PracticeSheet::save_request) {
        log.keep(record.clone());
        storage.submit(Job::Practice(PracticeJob::Save(record)));
    }
}

/// The frame's sheet intents at `now` (seconds since 1970 UTC), after its painting intents, each
/// applied as one of them is (`Canvas::by_mode`); One more lays the next lesson (`Start::Next`)
/// on the same sheet where it takes it (`day.rs`, the lesson
/// begun from `log` seeded from `seeds`, written by `storage`), else leaves for a new one.
pub(crate) fn apply(
    canvas: &mut Canvas,
    intents: Vec<PracticeIntent>,
    (log, seeds, storage): (&mut PracticeLog, &mut Seeds, &mut Storage),
    now: u64,
) {
    for intent in intents {
        let applied = canvas.by_mode(|session| practise(session, intent, now));
        if let (true, PracticeIntent::OneMore(viewport)) = (applied, intent) {
            let app = (&mut *log, &mut *seeds, &mut *storage);
            if !day::one_more(canvas, app, (now, &viewport)) {
                canvas.leaving = Leaving::Waiting;
                canvas.then = Then::Practice(day::named(canvas));
            }
        }
    }
}

/// Practice's part of the screen for one frame: the open sheet's, or on the home the path and
/// the options' notice.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Shown {
    sheet: Option<PracticeProjection>,
    path: Option<PathProjection>,
    notice: Option<OptionsNotice>,
}

impl Shown {
    /// What `screen` shows of practice, the path from `log`.
    pub(crate) fn of(screen: &Screen, log: &PracticeLog) -> Self {
        match screen {
            Screen::Canvas(canvas) => Self {
                sheet: PracticeProjection::of(canvas.session()),
                ..Self::default()
            },
            Screen::Gallery(_, options) => Self {
                path: Some(log.path()),
                notice: options.notice(),
                ..Self::default()
            },
            Screen::Opening(_) => Self::default(),
        }
    }

    /// The hooks `screens` fill for it in one frame.
    pub(crate) fn hooked<'a>(&'a self, screens: &'a mut PracticeScreens) -> Hooked<'a> {
        let path = self.path.as_ref().map(|path| (path, self.notice));
        screens.hooked((self.sheet.as_ref(), path))
    }
}

impl Runtime {
    /// The open practice sheet, while the canvas shows one.
    #[must_use]
    pub fn practice(&self) -> Option<PracticeProjection> {
        PracticeProjection::of(self.screen.canvas()?.session())
    }

    /// The path (S122: the home's practice column, the path page, the practice book), while the
    /// home shows.
    #[must_use]
    pub fn path(&self) -> Option<PathProjection> {
        matches!(self.screen, Screen::Gallery(..)).then(|| self.practice.path())
    }
}
