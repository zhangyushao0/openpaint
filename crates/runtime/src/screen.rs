//! What the screen shows and how it changes with the library's answers: opening an artwork
//! onto the canvas, leaving the canvas for the gallery once the artwork is stored, and the
//! gallery's calls on the storage worker.

use openpaint_app::{Gallery, LibraryAnswer, LibraryRequest, PortError, ScreenProjection, Toolbox};
use openpaint_core::Artwork;
use openpaint_render::Core;
use openpaint_ui::ScreenArea;

use crate::canvas::{Canvas, Content, Leaving, blank_size};
use crate::error::RuntimeError;
use crate::storage::{Done, Job, LibraryCall, Opened, Storage};

pub(crate) enum Screen {
    /// An artwork is about to show: waiting for the library (`None`) or the canvas area.
    Opening(Option<Content>),
    /// Boxed: a canvas holds its GPU target and is much larger than the other screens.
    Canvas(Box<Canvas>),
    Gallery(Gallery),
}

impl Screen {
    /// What the screen offers; on the canvas with the toolbox's brush, size and opacity.
    pub(crate) fn projection(&self, toolbox: &Toolbox) -> ScreenProjection {
        match self {
            Self::Opening(_) => ScreenProjection::Starting,
            Self::Canvas(canvas) => {
                let session = canvas.session().projection();
                ScreenProjection::Canvas(session.with_brush(toolbox.projection()))
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

    /// Shows `content` on a canvas once the area is known, drawing with the toolbox's brush.
    pub(crate) fn open(
        &mut self,
        core: &Core,
        area: Option<ScreenArea>,
        toolbox: &Toolbox,
    ) -> Result<(), RuntimeError> {
        if let (Self::Opening(Some(content)), Some(area)) = (&*self, area)
            && let Some(canvas) = Canvas::open(core, &area, content, toolbox.style()?)?
        {
            *self = Self::Canvas(Box::new(canvas));
        }
        Ok(())
    }

    /// Applies what the storage worker finished; the stored toolbox goes to `toolbox`.
    pub(crate) fn answer(&mut self, done: Done, toolbox: &mut Toolbox) {
        match done {
            Done::Started(opened, editor) => {
                if let Some(state) = editor
                    && !toolbox.restore(&state)
                {
                    log::warn!("part of the stored toolbox did not fit the brushes");
                }
                *self = started(opened);
            }
            Done::EditorSaved(result) => {
                if let Err(error) = result {
                    log::warn!("storing the toolbox failed: {error}");
                }
            }
            Done::Saved(version, result) => {
                match &result {
                    Ok(()) => log::debug!("saved"),
                    Err(error) => log::error!("saving failed: {error}"),
                }
                if let Some(canvas) = self.canvas_mut() {
                    canvas.session_mut().save_finished(&version, &result);
                }
            }
            Done::Left {
                version,
                result,
                closed,
            } => self.left(version.as_ref(), &result, closed),
            Done::Loaded(Ok(project)) => {
                *self = Self::Opening(Some(Content::Stored(project.artwork)))
            }
            Done::Created(artwork, Ok(id)) => {
                log::info!("new artwork {}", id.get());
                *self = Self::Opening(Some(Content::Stored(artwork)));
            }
            Done::Listed(listed) => self.gallery_answer(LibraryAnswer::Listed(listed)),
            Done::Changed(result) => self.gallery_answer(LibraryAnswer::Changed(result)),
            Done::Loaded(Err(error)) => self.gallery_answer(LibraryAnswer::OpenFailed(error)),
            Done::Created(_, Err(error)) => {
                self.gallery_answer(LibraryAnswer::CreateFailed(error));
            }
        }
    }

    /// The canvas's last version and thumbnail were handed to the store: on success the
    /// gallery replaces the canvas (its history goes with it); on failure the canvas stays and
    /// shows that saving failed.
    fn left(&mut self, version: Option<&Artwork>, result: &Result<(), PortError>, closed: bool) {
        if let Err(error) = result {
            log::error!("saving before leaving failed: {error}");
        }
        let Some(canvas) = self.canvas_mut() else {
            return;
        };
        if let Some(version) = version {
            canvas.session_mut().save_finished(version, result);
        }
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
    pub(crate) fn progress(
        &mut self,
        core: &Core,
        storage: &mut Storage,
        area: Option<ScreenArea>,
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Canvas(canvas) => {
                canvas.evaluate(core)?;
                match canvas.leaving {
                    Leaving::Staying => {
                        if let Some(version) = canvas.session_mut().save_request() {
                            storage.submit(Job::Save(version));
                        }
                    }
                    Leaving::Waiting if !canvas.session().saving() => {
                        let version = canvas.session_mut().leave_request();
                        storage.submit(leave_job(canvas, core, version, true));
                        canvas.leaving = Leaving::Storing;
                    }
                    Leaving::Waiting | Leaving::Storing => {}
                }
            }
            Self::Gallery(gallery) => {
                while let Some(request) = gallery.request() {
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
/// thumbnail of what the canvas shows; `close` for the gallery. A thumbnail that cannot be made
/// is left out (it is derived data).
pub(crate) fn leave_job(
    canvas: &Canvas,
    core: &Core,
    version: Option<Artwork>,
    close: bool,
) -> Job {
    let thumbnail = canvas
        .thumbnail(core)
        .inspect_err(|error| log::error!("no thumbnail: {error}"))
        .ok();
    Job::Leave {
        version,
        thumbnail,
        close,
    }
}

fn started(opened: Opened) -> Screen {
    match opened {
        Opened::Last(artwork) => Screen::Opening(Some(Content::Stored(artwork))),
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

/// The store's job for a gallery request; a new artwork is blank and fills the canvas `area`.
fn gallery_job(request: LibraryRequest, area: Option<ScreenArea>) -> Result<Job, RuntimeError> {
    Ok(Job::Library(match request {
        LibraryRequest::Create(title) => {
            let area = area.ok_or(RuntimeError::Render(
                openpaint_render::RenderError::Invalid("the canvas area is unknown"),
            ))?;
            let artwork = Artwork::new(blank_size(&area)?);
            return Ok(Job::Create { artwork, title });
        }
        LibraryRequest::List => LibraryCall::List,
        LibraryRequest::Open(id) => LibraryCall::Load(id),
        LibraryRequest::Rename(id, title) => LibraryCall::Rename(id, title),
        LibraryRequest::Duplicate(id) => LibraryCall::Duplicate(id),
        LibraryRequest::Delete(id) => LibraryCall::Delete(id),
    }))
}
