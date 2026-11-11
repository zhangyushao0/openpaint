//! A lesson of the path started (S122): the store's job for what the home, the path page or a
//! scenario asks (`openpaint_practice_app::Start`), the lesson the log starts
//! (`PracticeLog::start`: the next one offered, the review, a skill's, a unit's full lesson, the
//! first), seeded from the strokes' seed source, on a sheet for the day or a test, onto today's
//! sheet where that takes it (`day.rs`). At S122's integration this is the one way a lesson
//! starts: `start_job`, `drill_job` and `Next` go, One more starts `Start::Next`.

use openpaint_app::Title;
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{Givens, Layout, Purpose, Sheet};
use openpaint_practice_app::{PracticeLog, Start};

use super::day;
use super::jobs::{PracticeJob, seed};
use crate::error::RuntimeError;
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
