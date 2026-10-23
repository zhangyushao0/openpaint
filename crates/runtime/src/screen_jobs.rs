//! The store's jobs the screen hands over: leaving the canvas, and the gallery's calls (a new
//! practice session among them); and the gallery's intents applied in order.

use openpaint_app::{Gallery, LibraryRequest, PracticeLog};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_render::Core;
use openpaint_ui::CanvasArea;

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::practice::start_job;
use crate::seeds::Seeds;
use crate::storage::{Job, LibraryCall};

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
    if canvas.session().is_practice() {
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
/// canvas `area`; a new practice session the next of `practice`, seeded from `seeds`, at `now`.
pub(super) fn gallery_job(
    request: LibraryRequest,
    area: Option<CanvasArea>,
    (practice, seeds, now): (&PracticeLog, &mut Seeds, u64),
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
        LibraryRequest::Practice(stage) => return start_job(practice, stage, seeds, now),
    }))
}
