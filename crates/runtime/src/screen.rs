//! What the screen shows and how it changes with the library's answers: opening an artwork
//! onto the canvas, leaving the canvas for the gallery once the artwork is stored, and the
//! gallery's calls on the storage worker.

use std::sync::Arc;

use openpaint_app::{
    Clipboard, Gallery, LibraryAnswer, LibraryRequest, PortError, ScreenProjection, Toolbox,
    Transfers,
};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_render::Core;
use openpaint_ui::CanvasArea;

use crate::canvas::{Canvas, Content, Held, Leaving};
use crate::display::{CacheJob, CacheWorker};
use crate::error::RuntimeError;
use crate::storage::{Done, Job, LibraryCall, Loaded, Opened, Storage, Stored};

/// The workers a frame hands calls to: the store and the display cache (S54).
pub(crate) type Workers<'a> = (&'a mut Storage, &'a mut CacheWorker);

pub(crate) enum Screen {
    /// An artwork is about to show: waiting for the library (`None`) or the canvas area.
    Opening(Option<Content>),
    /// Boxed: a canvas holds its GPU target and is much larger than the other screens.
    Canvas(Box<Canvas>),
    Gallery(Gallery),
}

impl Screen {
    /// What the screen offers; on the canvas with the toolbox's brush, size and opacity, the
    /// export and reference flows and whether the clipboard can paste.
    pub(crate) fn projection(
        &self,
        toolbox: &Toolbox,
        (transfers, clipboard): (&Transfers, &Clipboard),
    ) -> ScreenProjection {
        match self {
            Self::Opening(_) => ScreenProjection::Starting,
            Self::Canvas(canvas) => {
                let session = canvas.session().projection();
                ScreenProjection::Canvas(Box::new(
                    session
                        .with_brush(toolbox.projection())
                        .with_transfers(transfers.projection())
                        .with_clipboard(clipboard.canvas()),
                ))
            }
            Self::Gallery(gallery) => ScreenProjection::Gallery(gallery.projection()),
        }
    }

    pub(crate) fn canvas(&self) -> Option<&Canvas> {
        match self {
            Self::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }

    pub(crate) fn canvas_mut(&mut self) -> Option<&mut Canvas> {
        match self {
            Self::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }

    pub(crate) fn gallery_mut(&mut self) -> Option<&mut Gallery> {
        match self {
            Self::Gallery(gallery) => Some(gallery),
            _ => None,
        }
    }

    /// Shows `content` on a canvas once the area is known, drawing with the toolbox's brush.
    pub(crate) fn open(
        &mut self,
        core: &Core,
        area: Option<CanvasArea>,
        toolbox: &Toolbox,
    ) -> Result<(), RuntimeError> {
        if let (Self::Opening(Some(content)), Some(area)) = (&*self, area)
            && let Some(canvas) = Canvas::open(core, &area, content, toolbox)?
        {
            *self = Self::Canvas(Box::new(canvas));
        }
        Ok(())
    }

    /// Applies what the storage worker finished; the stored toolbox goes to `toolbox`. After a
    /// save the canvas's pixels may go to the display cache (`cache`, from `core`).
    pub(crate) fn answer(
        &mut self,
        done: Done,
        (toolbox, core, cache): (&mut Toolbox, &Arc<Core>, &mut CacheWorker),
    ) {
        match done {
            Done::Started(opened, editor) => {
                if let Some(state) = editor
                    && !toolbox.restore(&state)
                {
                    log::warn!("part of the stored toolbox did not fit the brushes");
                }
                for name in toolbox.fell_back() {
                    log::warn!("the stored brush {name} is refused; its built-in is back");
                }
                for key in toolbox.dropped() {
                    log::warn!("the stored brush sets list {key} for no brush or again; dropped");
                }
                *self = started(opened);
            }
            Done::EditorSaved(result) => {
                if let Err(error) = result {
                    log::warn!("storing the toolbox failed: {error}");
                }
            }
            Done::Saved(version, result) => self.saved(&version, result, (core, cache)),
            Done::Left {
                version,
                result,
                closed,
            } => self.left(version.as_ref(), (&result, closed), (core, cache)),
            Done::Loaded(Ok(loaded)) => *self = opening(*loaded),
            Done::Created(artwork, title, Ok(id)) => {
                log::info!("new artwork {}", id.get());
                let held = Held {
                    project: id,
                    saved: None,
                    cached: None,
                    picture: None,
                };
                *self = Self::Opening(Some(Content::Stored(artwork, title, Box::new(held))));
            }
            Done::Listed(listed) => self.gallery_answer(LibraryAnswer::Listed(listed)),
            Done::Changed(result) => self.gallery_answer(LibraryAnswer::Changed(result)),
            Done::Restored(result) => self.gallery_answer(LibraryAnswer::Restored(result)),
            Done::FileSaved(result) => self.gallery_answer(LibraryAnswer::FileSaved(result)),
            Done::FileOpened(result) => self.gallery_answer(LibraryAnswer::FileOpened(result)),
            Done::Loaded(Err(error)) => self.gallery_answer(LibraryAnswer::OpenFailed(error)),
            Done::Created(_, _, Err(error)) => {
                self.gallery_answer(LibraryAnswer::CreateFailed(error));
            }
        }
    }

    /// An autosave finished: the session learns how, and once it stored the canvas's version
    /// its pixels may go to the display cache.
    fn saved(
        &mut self,
        version: &Artwork,
        result: Result<Option<Stored>, PortError>,
        (core, cache): (&Arc<Core>, &mut CacheWorker),
    ) {
        match &result {
            Ok(_) => log::debug!("saved"),
            Err(error) => log::error!("saving failed: {error}"),
        }
        if let Some(canvas) = self.canvas_mut() {
            canvas
                .session_mut()
                .save_finished(version, &result.clone().map(drop));
            if let Ok(Some(stored)) = result {
                canvas.stored(version, stored);
                canvas.keep_cache(core, cache);
            }
        }
    }

    /// The canvas's last version and thumbnail were handed to the store: its pixels may go to
    /// the display cache (complete, as the thumbnail evaluated it); on success the gallery
    /// replaces the canvas (its history goes with it); on failure the canvas stays and shows
    /// that saving failed.
    fn left(
        &mut self,
        version: Option<&Artwork>,
        (result, closed): (&Result<Option<Stored>, PortError>, bool),
        (core, cache): (&Arc<Core>, &mut CacheWorker),
    ) {
        if let Err(error) = result {
            log::error!("saving before leaving failed: {error}");
        }
        let Some(canvas) = self.canvas_mut() else {
            return;
        };
        if let Some(version) = version {
            canvas
                .session_mut()
                .save_finished(version, &result.clone().map(drop));
            if let Ok(Some(stored)) = result {
                canvas.stored(version, *stored);
            }
        }
        canvas.keep_cache(core, cache);
        if closed {
            log::info!("left the artwork for the gallery");
            *self = Self::Gallery(Gallery::new());
        } else {
            canvas.leaving = Leaving::Staying;
        }
    }

    fn gallery_answer(&mut self, answer: LibraryAnswer) {
        if let Self::Gallery(gallery) = self {
            gallery.answer(answer);
        } else {
            log::warn!("a gallery answer arrived after the gallery closed");
        }
    }

    /// Everything after the frame's intents: the canvas is evaluated and its unsaved version or
    /// its leaving handed to the store; the gallery's calls go to the store in order.
    /// `now` is the wall clock's seconds since 1970 UTC, for a version kept on leaving.
    pub(crate) fn progress(
        &mut self,
        core: &Arc<Core>,
        (storage, cache): Workers<'_>,
        (area, now): (Option<CanvasArea>, u64),
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Canvas(canvas) => {
                canvas.evaluate(core)?;
                canvas.keep_cache(core, cache);
                match canvas.leaving {
                    Leaving::Staying => {
                        if let Some(version) = canvas.session_mut().save_request() {
                            storage.submit(Job::Save(version));
                        }
                    }
                    Leaving::Waiting if !canvas.session().saving() => {
                        let version = canvas.session_mut().leave_request();
                        storage.submit(leave_job(canvas, core, version, (now, true)));
                        canvas.leaving = Leaving::Storing;
                    }
                    Leaving::Waiting | Leaving::Storing => {}
                }
            }
            Self::Gallery(gallery) => {
                while let Some(request) = gallery.request() {
                    // A deleted artwork's cached canvas goes after any write still queued.
                    if let LibraryRequest::Delete(id) = &request {
                        cache.submit(CacheJob::Delete(*id));
                    }
                    match gallery_job(request, area) {
                        Ok(job) => storage.submit(job),
                        Err(error) => {
                            log::error!("no blank artwork: {error}");
                            gallery.answer(LibraryAnswer::CreateFailed(PortError::Stopped));
                        }
                    }
                }
            }
            Self::Opening(_) => {}
        }
        Ok(())
    }
}

/// The store's job for leaving `canvas`: `version` (from `Session::leave_request`) and the
/// thumbnail of what the canvas shows (every tile waiting evaluated first, S49); `close` for the
/// gallery. A thumbnail that cannot be made is left out (it is derived data).
pub(crate) fn leave_job(
    canvas: &mut Canvas,
    core: &Core,
    version: Option<Artwork>,
    (kept_at, close): (u64, bool),
) -> Job {
    let thumbnail = canvas
        .thumbnail(core)
        .inspect_err(|error| log::error!("no thumbnail: {error}"))
        .ok();
    Job::Leave {
        version,
        thumbnail,
        kept_at,
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

/// The canvas about to show what the library opened, with its cached canvas or gallery picture.
fn opening(loaded: Loaded) -> Screen {
    let Loaded { project, cached } = loaded;
    let held = Held {
        project: project.id,
        saved: Some(project.saved),
        cached,
        picture: project.thumbnail,
    };
    Screen::Opening(Some(Content::Stored(
        project.artwork,
        project.title,
        Box::new(held),
    )))
}

fn started(opened: Opened) -> Screen {
    match opened {
        Opened::Last(loaded) => opening(*loaded),
        Opened::Empty => Screen::Opening(Some(Content::Blank)),
        Opened::Failed(error) => {
            log::error!("opening the last artwork failed: {error}");
            Screen::Opening(Some(Content::BlankAfterFailedOpen))
        }
        Opened::Gallery(listed) => {
            let mut gallery = Gallery::listed(Vec::new());
            gallery.answer(LibraryAnswer::Listed(listed));
            Screen::Gallery(gallery)
        }
    }
}

/// The store's job for a gallery request; a new artwork is blank, of the size chosen for the
/// canvas `area`.
fn gallery_job(request: LibraryRequest, area: Option<CanvasArea>) -> Result<Job, RuntimeError> {
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
