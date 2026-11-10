//! The practice options beside the gallery (`openpaint_practice_app::Options`, D112): their
//! intents applied after the gallery's, their requests turned into the store's jobs (a new
//! session, drill or copy holding the gallery until its sheet opens), the file a picker of theirs
//! gave, and the store's answers to their pickers.

use openpaint_app::{Gallery, ImageSource, LibraryAnswer, PortError};
use openpaint_practice::Purpose;
use openpaint_practice_app::{Options, OptionsFile, OptionsIntent, OptionsRequest, PracticeLog};

use super::jobs::{PracticeJob, copy_job, drill_job, start_job};
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
        let creates = matches!(
            request,
            OptionsRequest::Practice(_) | OptionsRequest::Drill(_) | OptionsRequest::Copy(_)
        );
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

/// The store's job for an options request: a new session the next of `log`, seeded from `seeds`,
/// at `now`.
pub(crate) fn options_job(
    request: OptionsRequest,
    (log, seeds, now): (&PracticeLog, &mut Seeds, u64),
) -> Result<Job, RuntimeError> {
    Ok(Job::Practice(match request {
        OptionsRequest::Practice(stage) => {
            return start_job(log, (stage, None, Purpose::Day), seeds, now);
        }
        OptionsRequest::Drill(family) => {
            return drill_job(log, (family, Purpose::Day), seeds, now);
        }
        OptionsRequest::Copy(picture) => return copy_job(log, &picture, seeds, now),
        OptionsRequest::RenameCopy(sheet, title) => PracticeJob::Rename(sheet, title),
        OptionsRequest::RemoveCopy(sheet) => {
            let copy = (log.of_canvas(sheet))
                .filter(|copy| copy.is_copy())
                .ok_or(RuntimeError::NotNow("a colour copy of that sheet"))?;
            PracticeJob::Remove(copy.clone())
        }
        OptionsRequest::RemovePicture(item) => PracticeJob::RemovePicture(item),
        OptionsRequest::PictureTier(item, tier) => PracticeJob::PictureTier(item, tier),
        OptionsRequest::ReadSheets(ids) => {
            let records = (ids.iter())
                .filter_map(|id| log.session(*id))
                .filter(|record| record.canvas != 0)
                .cloned()
                .collect();
            PracticeJob::Read(records)
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
