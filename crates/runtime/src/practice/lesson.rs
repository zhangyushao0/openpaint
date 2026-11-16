//! A lesson of the path started (S122), the one way a lesson starts: the store's job for what
//! the home, the path page, One more, a scenario or the test chore asks
//! (`openpaint_practice_app::Start`), the lesson the log starts (`PracticeLog::start`: the next
//! one offered, the review, a skill's, a re-test, the first), seeded from the strokes'
//! seed source, on a sheet for the day or a test, onto today's sheet where that takes it
//! (`day.rs`); and a project begun the same way on a sheet of its own: a long colour copy of a
//! picture chosen, a drawing from life against a photo of one's own (S151), a cold-warm copy of a
//! picture chosen (S154).

use openpaint_app::{GalleryIntent, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{Givens, ItemRef, Layout, Purpose, Sheet};
use openpaint_practice_app::{PracticeLog, Start};

use super::day;
use super::jobs::PracticeJob;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::seeds::Seeds;
use crate::storage::Job;

/// The job that stores the new sheet of `record`, titled `title`, of the size its layout asks (a
/// day's laid ahead, S121c).
pub(super) fn started(log: &PracticeLog, record: Sheet, title: Title) -> Result<Job, RuntimeError> {
    let [width, height] = Layout::laid_ahead(&record);
    let sheet = Artwork::new(CanvasSize::new(width, height)?);
    let givens = (record.lesson()).map_or_else(Givens::default, |lesson| {
        Givens::of_lesson(lesson, log.packs())
    });
    let printed = Givens::laid(std::slice::from_ref(&givens), &record)
        .ok_or(RuntimeError::NotNow("a lesson's givens"))?;
    Ok(Job::Practice(PracticeJob::Start {
        sheet,
        title,
        record,
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

/// The job that begins a long colour copy of `picture` (the user chose it) from the packs of
/// `log`, seeded from `seeds`, at `now`.
///
/// # Errors
/// `NotNow` when no pack kept holds the picture.
pub(crate) fn copy_job(
    log: &PracticeLog,
    picture: &ItemRef,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (copy, title) =
        (log.begin_copy(picture, seed, now)).ok_or(RuntimeError::NotNow("a picture to copy"))?;
    log::info!(
        "colour copy {} of {}/{}",
        copy.id,
        picture.pack,
        picture.item
    );
    started(log, copy, title)
}

/// The job that begins a drawing from life against `photo` (S151: one of the user's own
/// pictures, chosen) from the packs of `log`, seeded from `seeds`, at `now`.
///
/// # Errors
/// `NotNow` when the user's own pictures hold no such photo.
pub(crate) fn life_job(
    log: &PracticeLog,
    photo: &ItemRef,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (drawing, title) = (log.begin_from_life(photo, seed, now))
        .ok_or(RuntimeError::NotNow("a photo of one's own"))?;
    log::info!("drawing from life {} of {}", drawing.id, photo.item);
    started(log, drawing, title)
}

/// The job that begins a cold-warm copy of `picture` (S154: the user chose it) from the packs of
/// `log`, seeded from `seeds`, at `now`.
///
/// # Errors
/// `NotNow` when no pack kept holds the picture.
pub(crate) fn cold_warm_job(
    log: &PracticeLog,
    picture: &ItemRef,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (copy, title) = (log.begin_cold_warm(picture, seed, now))
        .ok_or(RuntimeError::NotNow("a picture to copy"))?;
    log::info!(
        "cold-warm copy {} of {}/{}",
        copy.id,
        picture.pack,
        picture.item
    );
    started(log, copy, title)
}

/// A seed for a new sheet from the strokes' seed source.
pub(crate) fn seed(seeds: &mut Seeds) -> u64 {
    u64::from(seeds.next()) << 32 | u64::from(seeds.next())
}

impl Runtime {
    /// A lesson started from the home as `start` names it, for the day, as a tap there starts
    /// one: what a scenario's `lesson` step does. Its sheet opens once the store wrote it; the
    /// next lesson while today's is under way opens that one's sheet instead, as Continue does.
    ///
    /// # Errors
    /// `NotNow` when the home does not show, or there is nothing to start (`lesson_job`); `App`
    /// while the home opens something else.
    pub fn start_lesson(&mut self, start: Start) -> Result<(), RuntimeError> {
        let open = (self.practice.path().open).filter(|_| start == Start::Next);
        let gallery = (self.screen.gallery_mut()).ok_or(RuntimeError::NotNow("the home"))?;
        if let Some(open) = open {
            gallery.dispatch(GalleryIntent::Open(open.sheet))?;
            return Ok(());
        }
        let now = self.frame_clock.wall_seconds();
        let start = (start, Purpose::Day);
        let job = lesson_job(&self.practice, start, &mut self.seeds, now)?;
        self.storage.submit(job);
        Ok(())
    }
}
