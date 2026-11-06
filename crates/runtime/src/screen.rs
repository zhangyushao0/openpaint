//! What the screen shows and how it changes with the library's answers: opening an artwork
//! (or a practice sheet) onto the canvas, leaving the canvas for the gallery (or for the next
//! practice session) once the artwork is stored, and the gallery's calls on the storage worker.

use std::sync::Arc;

use openpaint_app::{
    Clipboard, Gallery, LibraryAnswer, LibraryRequest, PortError, PracticeLog, ScreenProjection,
    SheetView, Toolbox, Transfers,
};
use openpaint_core::Artwork;
use openpaint_render::Core;
use openpaint_ui::CanvasArea;

use crate::canvas::{Canvas, Content, Held, Leaving, Then};
use crate::display::{CacheJob, CacheWorker};
use crate::error::RuntimeError;
use crate::practice::{Next, PracticeJob};
use crate::seeds::Seeds;
use crate::storage::{Done, Job, Loaded, Opened, Storage, Stored};

#[path = "screen_jobs.rs"]
mod jobs;
use jobs::gallery_job;
pub(crate) use jobs::{dispatch_gallery, leave_job};

/// The workers a frame hands calls to: the store and the display cache.
pub(crate) type Workers<'a> = (&'a mut Storage, &'a mut CacheWorker);
/// What applying the store's answers changes beside the screen.
pub(crate) type Answered<'a> = (
    &'a mut Toolbox,
    &'a Arc<Core>,
    &'a mut CacheWorker,
    &'a mut PracticeLog,
);

pub(crate) enum Screen {
    /// An artwork is about to show: waiting for the library (`None`) or the canvas area.
    Opening(Option<Content>),
    /// Boxed: a canvas holds its GPU target and is much larger than the other screens.
    Canvas(Box<Canvas>),
    Gallery(Gallery),
}

impl Screen {
    /// What the screen offers; on the canvas with the toolbox's brush, size and opacity, the
    /// export and reference flows and whether the clipboard can paste; in the gallery with the
    /// Today card.
    pub(crate) fn projection(
        &self,
        toolbox: &Toolbox,
        (transfers, clipboard, practice): (&Transfers, &Clipboard, &PracticeLog),
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
            Self::Gallery(gallery) => {
                ScreenProjection::Gallery(gallery.projection().with_today(practice.today()))
            }
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

    /// Shows `content` on a canvas once the area is known, drawing with the toolbox's brush; a
    /// practice sheet of `practice` opens as one, on a screen of `pixels_per_point` (a long
    /// colour copy choosing the toolbox's Hard Paint, S104).
    pub(crate) fn open(
        &mut self,
        core: &Core,
        (area, pixels_per_point): (Option<CanvasArea>, f32),
        (toolbox, practice): (&mut Toolbox, &PracticeLog),
    ) -> Result<(), RuntimeError> {
        if let (Self::Opening(Some(content)), Some(area)) = (&*self, area) {
            let sheet = match content {
                Content::Stored(artwork, title, held) => {
                    practice.of_canvas(held.project).map(|session| {
                        let before = practice.before(session);
                        let canvas = artwork.canvas();
                        let size = [canvas.width(), canvas.height()];
                        let view = SheetView::of(session, practice.packs(), (size, title));
                        let others = practice.tally_without(session.id);
                        (session.clone(), before, view, others)
                    })
                }
                Content::Blank | Content::BlankAfterFailedOpen => None,
            };
            let viewport = area.viewport(pixels_per_point);
            if let Some(canvas) = Canvas::open(core, (&area, &viewport), content, (toolbox, sheet))?
            {
                *self = Self::Canvas(Box::new(canvas));
            }
        }
        Ok(())
    }

    /// Applies what the storage worker finished; the stored toolbox goes to `toolbox` and the
    /// practice sessions to `practice`. After a save the canvas's pixels may go to the display
    /// cache (`cache`, from `core`). Returns the next practice session or drill to start now (One
    /// more, once the sheet left was stored).
    pub(crate) fn answer(
        &mut self,
        done: Done,
        (toolbox, core, cache, practice): Answered<'_>,
    ) -> Option<Next> {
        match done {
            Done::Started(opened, editor, (sessions, titles, packs)) => {
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
                *practice = PracticeLog::new(sessions, titles, packs);
                *self = started(opened);
            }
            Done::Picture(item, image) => {
                if let Some(canvas) = self.canvas_mut() {
                    canvas.session_mut().practice_picture_read(item, image);
                }
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
            } => return self.left(version.as_ref(), (&result, closed), (core, cache)),
            Done::Loaded(loaded) => *self = opening(*loaded),
            Done::Created(artwork, title, id) => {
                log::info!("new artwork {}", id.get());
                *self = Self::Opening(Some(Content::Stored(artwork, title, new_project(id))));
            }
            Done::Gallery(answer) => {
                if let LibraryAnswer::PackImported(Ok(pack))
                | LibraryAnswer::PictureAdded(Ok(pack)) = &answer
                {
                    practice.keep_pack(pack.clone());
                }
                self.gallery_answer(answer);
            }
            Done::Practice(done) => self.practice(done, practice),
        }
        None
    }

    /// One more's next session or drill could not be asked for: the gallery shows, saying a new
    /// one could not be made.
    pub(crate) fn not_started(&mut self) {
        self.gallery_answer(LibraryAnswer::CreateFailed(PortError::Stopped));
    }

    /// A gallery call's answer; a new artwork or session that failed while no gallery shows
    /// (One more) shows it with the failure.
    fn gallery_answer(&mut self, answer: LibraryAnswer) {
        match self {
            Self::Gallery(gallery) => gallery.answer(answer),
            _ if matches!(answer, LibraryAnswer::CreateFailed(_)) => {
                let mut gallery = Gallery::new();
                gallery.answer(answer);
                *self = Self::Gallery(gallery);
            }
            _ => log::warn!("a gallery answer arrived after the gallery closed"),
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
    /// replaces the canvas (its history goes with it), or for One more the next session's or
    /// drill's sheet is on its way (returns which); on failure the canvas stays and shows that
    /// saving failed.
    fn left(
        &mut self,
        version: Option<&Artwork>,
        (result, closed): (&Result<Option<Stored>, PortError>, bool),
        (core, cache): (&Arc<Core>, &mut CacheWorker),
    ) -> Option<Next> {
        if let Err(error) = result {
            log::error!("saving before leaving failed: {error}");
        }
        let canvas = self.canvas_mut()?;
        if let Some(version) = version {
            canvas
                .session_mut()
                .save_finished(version, &result.clone().map(drop));
            if let Ok(Some(stored)) = result {
                canvas.stored(version, *stored);
            }
        }
        canvas.keep_cache(core, cache);
        if !closed {
            canvas.leaving = Leaving::Staying;
            return None;
        }
        match canvas.then {
            Then::Gallery => {
                log::info!("left the artwork for the gallery");
                *self = Self::Gallery(Gallery::new());
                None
            }
            Then::Practice(next) => {
                log::info!("left the practice sheet for the next ({next:?})");
                *self = Self::Opening(None);
                Some(next)
            }
        }
    }

    /// Everything after the frame's intents: the canvas is evaluated and its unsaved version,
    /// its practice record or its leaving handed to the store; the gallery's calls go to the
    /// store in order. `now` is the wall clock's seconds since 1970 UTC, for a version kept on
    /// leaving and a new practice session (seeded from `seeds`, the next of `practice`).
    pub(crate) fn progress(
        &mut self,
        core: &Arc<Core>,
        (storage, cache): Workers<'_>,
        (area, now): (Option<CanvasArea>, u64),
        (practice, seeds): (&mut PracticeLog, &mut Seeds),
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Canvas(canvas) => {
                canvas.evaluate(core)?;
                canvas.keep_cache(core, cache);
                // A sheet being left takes its time to the store with it.
                if canvas.leaving != Leaving::Staying {
                    canvas.session_mut().practice_flush();
                }
                if let Some(session) = canvas.session_mut().practice_save_request() {
                    practice.keep(session.clone());
                    storage.submit(Job::Practice(PracticeJob::Save(session)));
                }
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
                    // A deleted artwork's or copy's cached canvas goes after any write queued.
                    if let LibraryRequest::Delete(id) | LibraryRequest::RemoveCopy(id) = &request {
                        cache.submit(CacheJob::Delete(*id));
                    }
                    let creates = matches!(
                        request,
                        LibraryRequest::Create(..)
                            | LibraryRequest::Practice(_)
                            | LibraryRequest::Drill(_)
                            | LibraryRequest::Copy(_)
                    );
                    match gallery_job(request, area, (practice, seeds, now)) {
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
            Self::Opening(_) => {}
        }
        Ok(())
    }
}

/// What a canvas opening project `id`, just made, holds for it: nothing to show yet.
fn new_project(id: openpaint_app::ProjectId) -> Box<Held> {
    Box::new(Held {
        project: id,
        saved: None,
        cached: None,
        picture: None,
    })
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
