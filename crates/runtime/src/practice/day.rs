//! The day's sheet in the runtime (S121c, `openpaint_practice_app::PracticeLog::day_sheet`): a
//! lesson started from the home or the options goes onto today's sheet, the store
//! reading its canvas first (the rows left must hold the lesson; a sheet from before holds only
//! the rows it uses) and falling back to a new sheet; One more appends the next lesson to the
//! open sheet in place where it takes it, its record and givens written in one call, the answer
//! rolling it back if the write failed.

use openpaint_app::{PortError, ProjectId, ProjectStore, Viewport};
use openpaint_practice::{Givens, Layout, Lesson, Sheet};
use openpaint_practice_app::{PracticeLog, PracticeProjection, Start, append_lesson};
use openpaint_practice_store::{Packs, append};
use openpaint_store::DisplayCache;

use super::jobs::{PracticeDone, PracticeJob, run, seed};
use crate::canvas::Canvas;
use crate::seeds::Seeds;
use crate::storage::{Done, Job, Kept, Storage, loaded};

/// `job` (a new sheet's `Start`) onto today's sheet of `log`, when `day` allows it (a day's
/// lesson started from the home or the options) and one is there.
pub(super) fn onto_day(log: &PracticeLog, job: Job, day: bool) -> Job {
    match (job, log.day_sheet().filter(|_| day)) {
        (Job::Practice(start @ PracticeJob::Start { .. }), Some(sheet)) => {
            Job::Practice(PracticeJob::Continue {
                day: sheet.clone(),
                start: Box::new(start),
            })
        }
        (job, _) => job,
    }
}

/// On the storage worker: `start`'s lesson onto today's sheet `day` when its canvas holds it, the
/// sheet becoming the open one; else `start` itself.
pub(super) fn continued(
    (store, cache, packs): (&mut dyn ProjectStore, Option<&DisplayCache>, Option<&Packs>),
    kept: &mut Kept,
    (day, start): (Sheet, PracticeJob),
) -> Done {
    if let PracticeJob::Start {
        session, givens, ..
    } = &start
        && let Some(lesson) = session.lessons.last()
        && let Some(done) = onto(store, cache, kept, day, (lesson.clone(), givens.clone()))
    {
        return done;
    }
    run((store, cache, packs), kept, start)
}

/// `lesson` and `givens` appended to `day` and written, its sheet loaded as the open one; none
/// when its canvas cannot be read or does not hold the lesson, or the write failed (nothing
/// written then).
fn onto(
    store: &mut dyn ProjectStore,
    cache: Option<&DisplayCache>,
    kept: &mut Kept,
    mut day: Sheet,
    (lesson, givens): (Lesson, Givens),
) -> Option<Done> {
    let id = ProjectId::new(day.canvas);
    let project = (store.load(id))
        .inspect_err(|error| log::warn!("today's practice sheet {} not read: {error}", day.id))
        .ok()?;
    let canvas = project.artwork.canvas();
    if !Layout::fits(
        &day,
        lesson.exercises.len(),
        [canvas.width(), canvas.height()],
    ) {
        log::info!(
            "the lesson does not fit on practice sheet {}: a new sheet",
            day.id
        );
        return None;
    }
    let givens = day.append(lesson, givens);
    (append(store, &day, &givens))
        .inspect_err(|error| log::error!("practice sheet {} took no lesson: {error}", day.id))
        .ok()?;
    kept.open = Some(id);
    kept.changed = false;
    Some(Done::Practice(match loaded((store, cache), project) {
        Ok(loaded) => PracticeDone::Continued {
            session: day,
            loaded,
        },
        Err(error) => {
            log::error!("practice sheet {} not opened: {error}", day.id);
            PracticeDone::Failed(error)
        }
    }))
}

/// One more on `canvas`'s sheet: `start`'s lesson (One more's and the Pomodoro's chain: what its
/// end names, `named`; the practice bench: the next skill of its unit) seeded from `seeds`, begun
/// at `now` by `log` (the log holding the finished record, `save_record`), appended in place
/// where the sheet takes it, the view gliding to it in `viewport`, its record kept in `log` and
/// written by `storage`. Returns whether it was.
pub(crate) fn one_more(
    canvas: &mut Canvas,
    (log, seeds, storage): (&mut PracticeLog, &mut Seeds, &mut Storage),
    (start, now, viewport): (Start, u64, &Viewport),
) -> bool {
    let seed = seed(seeds);
    let begun = log.start(start, seed, now);
    let begun = begun.map(|(sheet, _)| sheet);
    let Some(lesson) = begun.and_then(|sheet| sheet.lessons.into_iter().last()) else {
        return false;
    };
    let givens = Givens::of_lesson(&lesson, log.packs());
    let session = canvas.session_mut();
    let Some((sheet, givens)) = append_lesson(session, (lesson, givens), log, viewport) else {
        return false;
    };
    log::info!(
        "practice sheet {} took lesson {} in place, seeded {seed:016x}",
        sheet.id,
        sheet.lessons.len() - 1
    );
    log.keep(sheet.clone());
    storage.submit(Job::Practice(PracticeJob::Append { sheet, givens }));
    true
}

/// The store's answer to One more's lesson appended to sheet `id` (then holding `lessons`): on a
/// failure the log forgets the lesson and the open sheet rolls it back.
pub(crate) fn appended(
    (canvas, log): (Option<&mut Canvas>, &mut PracticeLog),
    (id, lessons): (u64, usize),
    result: &Result<(), PortError>,
) {
    if let Err(error) = result {
        log::error!("the lesson appended to practice sheet {id} was not stored: {error}");
        if let Some(kept) = log
            .session(id)
            .filter(|sheet| sheet.lessons.len() == lessons)
        {
            let mut kept = kept.clone();
            kept.lessons.pop();
            log.keep(kept);
        }
    }
    if let Some(canvas) = canvas {
        openpaint_practice_app::append_stored(canvas.session_mut(), id, result.is_ok());
    }
}

/// The lesson One more starts from `canvas`'s sheet: what its end names (`End::next`, a skill of
/// a unit ahead of the learner's again), the next lesson where it names none.
pub(crate) fn named(canvas: &Canvas) -> Start {
    (PracticeProjection::of(canvas.session()))
        .and_then(|shown| shown.end?.next)
        .map_or(Start::Next, |offer| Start::Skill(offer.skill))
}
