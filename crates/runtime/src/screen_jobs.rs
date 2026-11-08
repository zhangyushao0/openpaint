//! The store's jobs the screen hands over: leaving the canvas, and the gallery's calls; the
//! gallery's intents applied in order; and practice's answers from the store.

use openpaint_app::{Gallery, LibraryAnswer, LibraryRequest, PortError};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice_app::{Options, PracticeLog};
use openpaint_render::Core;
use openpaint_ui::CanvasArea;

use super::{Content, Screen, new_project};
use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::practice::{PracticeDone, sheet};
use crate::storage::{Job, LibraryCall};

impl Screen {
    /// A practice session started (its sheet opens), failed to, or its record was written; a
    /// colour copy renamed or removed; a pack imported; the user's own pictures changed; a pack's
    /// picture read for the open sheet. A change that failed leaves everything as it was and the
    /// gallery says so.
    pub(super) fn practice(&mut self, done: PracticeDone, practice: &mut PracticeLog) {
        let failed = |screen: &mut Self, error: PortError| {
            screen.gallery_answer(LibraryAnswer::Changed(Err(error)));
        };
        match done {
            PracticeDone::Started {
                sheet,
                title,
                id,
                session,
            } => {
                log::info!("practice session {} on project {}", session.id, id.get());
                practice.keep(session);
                practice.name(id, title.clone());
                *self = Self::Opening(Some(Content::Stored(sheet, title, new_project(id))));
            }
            PracticeDone::Failed(error) => {
                self.gallery_answer(LibraryAnswer::CreateFailed(error));
            }
            PracticeDone::Saved(Err(error)) => {
                log::error!("storing the practice session failed: {error}");
            }
            PracticeDone::Saved(Ok(())) => {}
            PracticeDone::Renamed(sheet, title, Ok(())) => practice.name(sheet, title),
            PracticeDone::Removed(id, Ok(())) => {
                log::info!("practice session {id} removed");
                practice.forget(id);
            }
            PracticeDone::Pictures(Ok(pack)) => practice.set_personal(pack),
            PracticeDone::Read(read) => practice.sheets_read(read),
            PracticeDone::PackImported(result) => {
                if let Ok(pack) = &result {
                    practice.keep_pack(pack.clone());
                }
                self.options_answered(|options| options.imported(&result));
            }
            PracticeDone::PictureAdded(result) => {
                if let Ok(pack) = &result {
                    practice.keep_pack(pack.clone());
                }
                self.options_answered(|options| options.added(&result));
            }
            PracticeDone::Picture(item, image) => {
                let open = self.canvas_mut().and_then(sheet);
                if let Some(sheet) = open {
                    sheet.picture_read(item, image);
                }
            }
            PracticeDone::Renamed(.., Err(error))
            | PracticeDone::Removed(_, Err(error))
            | PracticeDone::Pictures(Err(error)) => {
                log::error!("a practice change failed: {error}");
                failed(self, error);
            }
        }
    }

    /// The store finished a file the practice options' picker gave: they say how it went, and
    /// the gallery's next dialog may open.
    fn options_answered(&mut self, answer: impl FnOnce(&mut Options)) {
        match self.options_mut() {
            Some((gallery, options)) => {
                answer(options);
                gallery.file_done();
            }
            None => log::warn!("a gallery answer arrived after the gallery closed"),
        }
    }
}

/// The store's job for leaving `canvas`: `version` (from `Session::leave_request`) and the
/// thumbnail of what the canvas shows (every tile waiting evaluated first), with a version kept
/// at `kept_at`; `close` for the gallery. A practice sheet keeps neither picture nor version. A
/// thumbnail that cannot be made is left out (it is derived data).
pub(crate) fn leave_job(
    canvas: &mut Canvas,
    core: &Core,
    version: Option<Artwork>,
    (kept_at, close): (u64, bool),
) -> Job {
    if sheet(canvas).is_some() {
        return Job::Leave {
            version,
            thumbnail: None,
            kept_at: None,
            close,
        };
    }
    let thumbnail = canvas
        .thumbnail(core)
        .inspect_err(|error| log::error!("no thumbnail: {error}"))
        .ok();
    Job::Leave {
        version,
        thumbnail,
        kept_at: Some(kept_at),
        close,
    }
}

/// The gallery's intents in order; anything else from a gallery frame is a stray.
pub(crate) fn dispatch_gallery(gallery: &mut Gallery, intents: Vec<openpaint_app::Intent>) {
    for intent in intents {
        let result = match intent {
            openpaint_app::Intent::Gallery(intent) => gallery.dispatch(intent),
            other => {
                log::debug!("ignored in the gallery: {other:?}");
                Ok(())
            }
        };
        if let Err(error) = result {
            log::warn!("intent refused: {error}");
        }
    }
}

/// The store's job for a gallery request; a new artwork is blank, of the size chosen for the
/// canvas `area`.
pub(super) fn gallery_job(
    request: LibraryRequest,
    area: Option<CanvasArea>,
) -> Result<Job, RuntimeError> {
    Ok(Job::Library(match request {
        LibraryRequest::Create(title, preset) => {
            let area = area.ok_or(RuntimeError::Render(
                openpaint_render::RenderError::Invalid("the canvas area is unknown"),
            ))?;
            let [width, height] = preset.size(area.fit.size);
            let artwork = Artwork::new(CanvasSize::new(width, height)?);
            return Ok(Job::Create { artwork, title });
        }
        LibraryRequest::List => LibraryCall::List,
        LibraryRequest::Open(id) => LibraryCall::Load(id),
        LibraryRequest::Rename(id, title) => LibraryCall::Rename(id, title),
        LibraryRequest::Duplicate(id) => LibraryCall::Duplicate(id),
        LibraryRequest::Delete(id) => LibraryCall::Delete(id),
        LibraryRequest::Restore(id, version, title) => LibraryCall::Restore(id, version, title),
    }))
}
