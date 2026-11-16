//! The practice options beside the home (`openpaint_practice_app::Options`, D112): their
//! intents applied after the home's, their requests turned into the store's jobs (a new
//! lesson or copy holding the home until its sheet opens), the file a picker of theirs
//! gave, and the store's answers to their pickers.

use openpaint_app::{Gallery, ImageSource, LibraryAnswer, PortError};
use openpaint_practice::Purpose;
use openpaint_practice_app::{Options, OptionsFile, OptionsIntent, OptionsRequest, PracticeLog};

use super::jobs::PracticeJob;
use super::lesson::{copy_job, lesson_job, life_job};
use crate::display::{CacheJob, CacheWorker};
use crate::error::RuntimeError;
use crate::seeds::Seeds;
use crate::storage::{Job, Storage};

/// The frame's options intents beside `gallery`, in order, after the gallery's own.
pub(crate) fn dispatch_options(
    (gallery, options): (&mut Gallery, &mut Options),
    intents: Vec<OptionsIntent>,
) {
    for intent in intents {
        if let Err(error) = options.dispatch(intent, gallery) {
            log::warn!("intent refused: {error}");
        }
    }
}

/// The options' requests to the store in order; one that cannot be asked fails as the gallery's
/// would (a new sheet as not made, a change as not done). A colour copy removed takes its cached
/// canvas with it.
pub(crate) fn submit_options(
    (gallery, options): (&mut Gallery, &mut Options),
    (storage, cache): (&mut Storage, &mut CacheWorker),
    (log, seeds, now): (&PracticeLog, &mut Seeds, u64),
) {
    while let Some(request) = options.request() {
        if let OptionsRequest::RemoveCopy(id) = &request {
            cache.submit(CacheJob::Delete(*id));
        }
        let creates = request.opens();
        match options_job(request, (log, seeds, now)) {
            Ok(job) => storage.submit(job),
            Err(error) if creates => {
                log::error!("nothing to open: {error}");
                gallery.answer(LibraryAnswer::CreateFailed(PortError::Stopped));
            }
            Err(error) => {
                log::error!("a change was not asked: {error}");
                gallery.answer(LibraryAnswer::Changed(Err(PortError::Stopped)));
            }
        }
    }
}

/// The store's job for an options request: a lesson `log` starts, seeded from `seeds`,
/// at `now`.
pub(crate) fn options_job(
    request: OptionsRequest,
    (log, seeds, now): (&PracticeLog, &mut Seeds, u64),
) -> Result<Job, RuntimeError> {
    Ok(Job::Practice(match request {
        OptionsRequest::Lesson(start) => {
            return lesson_job(log, (start, Purpose::Day), seeds, now);
        }
        OptionsRequest::Copy(picture) => return copy_job(log, &picture, seeds, now),
        OptionsRequest::FromLife(photo) => return life_job(log, &photo, seeds, now),
        OptionsRequest::RenameCopy(sheet, title) => PracticeJob::Rename(sheet, title),
        OptionsRequest::RemoveCopy(sheet) => {
            let copy = (log.of_canvas(sheet))
                .filter(|copy| copy.is_copy())
                .ok_or(RuntimeError::NotNow("a project of that sheet"))?;
            PracticeJob::Remove(copy.clone())
        }
        OptionsRequest::RemovePicture(item) => PracticeJob::RemovePicture(item),
        OptionsRequest::PictureTier(item, tier) => PracticeJob::PictureTier(item, tier),
        OptionsRequest::ReadSheets(ids) => {
            let records = (ids.iter())
                .filter_map(|id| log.sheet(*id))
                .filter(|record| record.canvas != 0)
                .cloned()
                .collect();
            PracticeJob::Read(records)
        }
        OptionsRequest::Refit => PracticeJob::Refit(log.refit()),
        OptionsRequest::ReadCards(copies) => {
            PracticeJob::Cards(log.copies_of(&copies), log.offset())
        }
    }))
}

/// The options' picker asked with `tag` answered with `source` (none: the system could not give
/// it): the store's job for the file, or none, the options saying why.
pub(crate) fn chosen(
    options: &mut Options,
    (tag, source): (u32, Option<Box<dyn ImageSource>>),
) -> Option<Job> {
    let job = match options.chosen(tag, source)? {
        OptionsFile::Pack(source) => PracticeJob::ImportPack(source),
        OptionsFile::Picture(source, tier) => PracticeJob::AddPicture(source, tier),
    };
    Some(Job::Practice(job))
}
