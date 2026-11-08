//! Practice composed into the runtime (G9, D112): the painting crates know only a mode slot on the
//! session and the gallery's turn for pickers, and this module puts practice into them. A sheet
//! opens as the session's mode (`Canvas::open`); its intents reach it after the frame's painting
//! intents, One more leaving it for the next session or drill (`apply`); what the sheet asks of
//! the store and the workers, and what they answer, passes through `sheet`, the one place the
//! runtime reaches the sheet; the practice options take their turn beside the gallery
//! (`options.rs`); practice's part of the screen (`Shown`) fills the shell's hooks through its
//! screens (`openpaint_practice_ui::PracticeScreens`). The store's jobs are `jobs.rs` and
//! `files.rs`, Compare's worker `compare.rs`, the options' pictures `pack_pictures.rs`, the test
//! sessions of debug builds `test.rs`.

mod compare;
mod files;
mod jobs;
mod options;
mod pack_pictures;
mod test;

use openpaint_practice_app::{
    OptionsNotice, PracticeIntent, PracticeLog, PracticeProjection, PracticeSheet, TodayProjection,
    practise,
};
use openpaint_practice_ui::{Hooked, PracticeScreens};

use crate::canvas::{Canvas, Leaving, Then};
use crate::screen::Screen;
use crate::storage::{Job, Storage};

pub(crate) use compare::CompareWorker;
pub(crate) use jobs::{
    Next, PracticeDone, PracticeJob, copy_job, drill_job, gallery_list, run, seed, start_job,
    stopped,
};
pub(crate) use options::{chosen, dispatch_options, submit_options};

use crate::frame::Runtime;
pub(crate) use pack_pictures::PackPictureWorker;
pub use test::PracticeTest;

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
/// applied as one of them is (`Canvas::by_mode`); One more leaves the sheet for the next session,
/// or after a drill another drill of its kind.
pub(crate) fn apply(canvas: &mut Canvas, intents: Vec<PracticeIntent>, now: u64) {
    for intent in intents {
        let applied = canvas.by_mode(|session| practise(session, intent, now));
        if applied && intent == PracticeIntent::OneMore {
            let drill = sheet(canvas).and_then(|sheet| sheet.drill());
            canvas.leaving = Leaving::Waiting;
            canvas.then = Then::Practice(drill.map_or(Next::Session, Next::Drill));
        }
    }
}

/// Practice's part of the screen for one frame: the open sheet's, or in the gallery the Today
/// card and the options' notice.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Shown {
    sheet: Option<PracticeProjection>,
    today: Option<TodayProjection>,
    notice: Option<OptionsNotice>,
}

impl Shown {
    /// What `screen` shows of practice, the Today card from `log`.
    pub(crate) fn of(screen: &Screen, log: &PracticeLog) -> Self {
        match screen {
            Screen::Canvas(canvas) => Self {
                sheet: PracticeProjection::of(canvas.session()),
                ..Self::default()
            },
            Screen::Gallery(_, options) => Self {
                today: Some(log.today()),
                notice: options.notice(),
                ..Self::default()
            },
            Screen::Opening(_) => Self::default(),
        }
    }

    /// The hooks `screens` fill for it in one frame.
    pub(crate) fn hooked<'a>(&'a self, screens: &'a mut PracticeScreens) -> Hooked<'a> {
        let today = self.today.as_ref().map(|today| (today, self.notice));
        screens.hooked((self.sheet.as_ref(), today))
    }
}

impl Runtime {
    /// The open practice sheet, while the canvas shows one.
    #[must_use]
    pub fn practice(&self) -> Option<PracticeProjection> {
        PracticeProjection::of(self.screen.canvas()?.session())
    }

    /// The Today card, its options and the practice book, while the gallery shows.
    #[must_use]
    pub fn today(&self) -> Option<TodayProjection> {
        matches!(self.screen, Screen::Gallery(..)).then(|| self.practice.today())
    }
}
