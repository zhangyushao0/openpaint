//! A lesson of the path started (S122), the one way a lesson starts: the store's job for what
//! the home, the path page, One more, a scenario or the test chore asks
//! (`openpaint_practice_app::Start`), the lesson the log starts (`PracticeLog::start`: the next
//! one offered, the review, a skill's, a unit's full lesson, the first), seeded from the strokes'
//! seed source, on a sheet for the day or a test, onto today's sheet where that takes it
//! (`day.rs`).

use openpaint_app::Title;
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{Givens, Layout, Purpose, Sheet};
use openpaint_practice_app::{PracticeLog, Start};

use super::day;
use super::jobs::{PracticeJob, seed};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::seeds::Seeds;
use crate::storage::Job;

/// The job that stores `session`'s new sheet, titled `title`, of the size its layout asks (a
/// day's laid ahead, S121c).
pub(super) fn started(
    log: &PracticeLog,
    session: Sheet,
    title: Title,
) -> Result<Job, RuntimeError> {
    let [width, height] = Layout::laid_ahead(&session);
    let sheet = Artwork::new(CanvasSize::new(width, height)?);
    let givens = (session.lesson()).map_or_else(Givens::default, |lesson| {
        Givens::of_lesson(lesson, log.packs())
    });
    let printed = Givens::laid(std::slice::from_ref(&givens), &session)
        .ok_or(RuntimeError::NotNow("a lesson's givens"))?;
    Ok(Job::Practice(PracticeJob::Start {
        sheet,
        title,
        session,
        givens,
        printed,
    }))
}

/// The job that starts the lesson `start` names in `log` on a sheet for `purpose`, seeded from
/// `seeds`, at `now` (seconds since 1970 UTC).
///
/// # Errors
/// `NotNow` where there is nothing to start: no skill to practise, no redo to review, a skill
/// whose pictures are not kept, a unit not built.
pub(crate) fn lesson_job(
    log: &PracticeLog,
    (start, purpose): (Start, Purpose),
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (lesson, title) = (log.start(start, seed, now))
        .ok_or(RuntimeError::NotNow("a lesson to start with its pictures"))?;
    let lesson = Sheet { purpose, ..lesson };
    log::info!(
        "practice lesson {} ({start:?}) seeded {seed:016x}",
        lesson.id
    );
    let job = started(log, lesson, title)?;
    Ok(day::onto_day(log, job, purpose == Purpose::Day))
}

impl Runtime {
    /// A lesson started from the home as `start` names it, for the day, as a tap there starts
    /// one: what a scenario's `lesson` step does. Its sheet opens once the store wrote it.
    ///
    /// # Errors
    /// `NotNow` when the home does not show, or there is nothing to start (`lesson_job`).
    pub fn start_lesson(&mut self, start: Start) -> Result<(), RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the home"));
        }
        let now = self.frame_clock.wall_seconds();
        let start = (start, Purpose::Day);
        let job = lesson_job(&self.practice, start, &mut self.seeds, now)?;
        self.storage.submit(job);
        Ok(())
    }
}
