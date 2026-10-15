//! The open canvas and the display cache (S54, DECISIONS D81). Opening an artwork, its cached
//! canvas is put into the new target: when its key matches (the save that wrote the artwork's
//! head, this renderer, the target's format and size) the canvas is complete in its first frame
//! and nothing fills; otherwise it, or the gallery picture enlarged, shows under the tiles while
//! they fill as before. Once the canvas shows a version the library holds complete, its pixels
//! go to the cache worker, which writes them under that version's key.

use std::sync::Arc;
use std::time::Instant;

use openpaint_app::{ProjectId, SaveId, Thumbnail};
use openpaint_core::Artwork;
use openpaint_render::{CanvasTarget, Core};
use openpaint_store::{CacheKey, Cached};

use super::Canvas;
use crate::display::{CacheJob, CacheWorker};
use crate::storage::Stored;

/// What a canvas opening an artwork of the library shows before it is evaluated, and the key
/// its cache must match.
#[derive(Debug)]
pub(crate) struct Held {
    pub(crate) project: ProjectId,
    /// The save id the artwork's head holds; `None` for one just created.
    pub(crate) saved: Option<SaveId>,
    pub(crate) cached: Option<Cached>,
    /// Its gallery picture, shown enlarged when no cache fits.
    pub(crate) picture: Option<Thumbnail>,
}

/// The canvas's part in the display cache.
#[derive(Debug, Default)]
pub(crate) struct CacheState {
    /// The artwork's project, once it is in the library.
    project: Option<ProjectId>,
    /// The version the library holds, and the save id its head holds.
    stored: Option<(Artwork, SaveId)>,
    /// The key last handed to the worker or found matching at opening: written again only for
    /// another key.
    written: Option<CacheKey>,
}

/// The key the cache of `target`, evaluated by `core`, has when the artwork's head holds `saved`.
fn key(core: &Core, target: &CanvasTarget, saved: SaveId) -> CacheKey {
    CacheKey {
        saved,
        renderer: core.renderer_identity(),
        format: target.format_code(),
        size: target.size(),
    }
}

impl Canvas {
    /// Puts into the new target what `held` holds for it: the cached canvas complete when its
    /// key matches, else it or the gallery picture under the tiles while they fill. Logs which,
    /// and why a cache is not complete.
    pub(super) fn show_opened(&mut self, core: &Core, held: &Held) {
        let project = held.project.get();
        self.cache.project = Some(held.project);
        let wanted = held.saved.map(|saved| key(core, &self.target, saved));
        if let Some(saved) = held.saved {
            self.cache.stored = Some((self.session.artwork().clone(), saved));
        }
        let (format, size) = (self.target.format_code(), self.target.size());
        let why = match (&held.cached, wanted) {
            (None, _) => "none kept",
            (Some(cached), _) if (cached.key.format, cached.key.size) != (format, size) => {
                "another format or size"
            }
            (Some(cached), Some(wanted)) if cached.key == wanted => {
                let started = Instant::now();
                if let Err(error) = self.target.show_cached(core, &cached.pixels, true) {
                    log::error!("the display cache was not shown: {error}");
                    return;
                }
                self.evaluated = true;
                self.cache.written = Some(wanted);
                log::info!(
                    "display cache of project {project}: the canvas complete at once (uploaded \
                     in {} ms)",
                    started.elapsed().as_millis()
                );
                return;
            }
            (Some(cached), _) => {
                let why = if wanted.is_some_and(|wanted| wanted.renderer != cached.key.renderer) {
                    "another renderer"
                } else {
                    "another save"
                };
                match self.target.show_cached(core, &cached.pixels, false) {
                    Ok(()) => {
                        log::info!(
                            "display cache of project {project} not complete ({why}): shown \
                             while the canvas fills"
                        );
                        return;
                    }
                    Err(error) => {
                        log::error!("the display cache was not shown: {error}");
                        "not shown"
                    }
                }
            }
        };
        self.show_picture(core, held.picture.as_ref(), (project, why));
    }

    /// The gallery picture enlarged under the tiles while they fill, or the paper without one.
    fn show_picture(
        &mut self,
        core: &Core,
        picture: Option<&Thumbnail>,
        (project, why): (u64, &str),
    ) {
        let Some(picture) = picture else {
            log::info!("display cache of project {project}: {why}, no gallery picture: paper");
            return;
        };
        match self
            .target
            .show_picture(core, (picture.size(), picture.pixels()))
        {
            Ok(()) => log::info!(
                "display cache of project {project}: {why}; the gallery picture shown while the \
                 canvas fills"
            ),
            Err(error) => log::error!("the gallery picture was not shown: {error}"),
        }
    }

    /// The artwork's project, once it is in the library.
    pub(crate) const fn project(&self) -> Option<ProjectId> {
        self.cache.project
    }

    /// The library holds `version` now, as `stored` says (a save completed).
    pub(crate) fn stored(&mut self, version: &Artwork, stored: Stored) {
        self.cache.project = Some(stored.project);
        self.cache.stored = Some((version.clone(), stored.saved));
    }

    /// Once the canvas shows the version the library holds complete (every tile evaluated, no
    /// stroke drawn, no preview, nothing changed since) and its cache under that version's key
    /// was not handed over yet, hands `worker` a copy of the target to write: nothing waits
    /// here. One at a time; a later frame takes the next.
    pub(crate) fn keep_cache(&mut self, core: &Arc<Core>, worker: &mut CacheWorker) {
        if !worker.on() || worker.busy() {
            return;
        }
        let (Some(project), Some((version, saved))) = (self.cache.project, &self.cache.stored)
        else {
            return;
        };
        let complete = self.evaluated
            && self.changed_from.is_none()
            && !self.target.pending()
            && self.session.active().is_none()
            && self.session.shown().same_version(version);
        let key = key(core, &self.target, *saved);
        if !complete || self.cache.written == Some(key) {
            return;
        }
        let copy = self.target.copy_out(core);
        worker.submit(CacheJob::Write {
            core: Arc::clone(core),
            project,
            key,
            copy,
        });
        self.cache.written = Some(key);
    }
}
