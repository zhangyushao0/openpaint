//! `bench open-cold|open`: the artwork left for the gallery and opened again from its card, as a
//! hand does, timed.

use openpaint_app::{GalleryIntent, Intent, ProjectId};

use super::hand::strokes;
use super::{Watch, reached};
use crate::display::CacheJob;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::screen::Screen;

impl Runtime {
    /// `open-cold` (the display cache deleted first) and `open`.
    pub(crate) fn bench_open(
        &mut self,
        cold: bool,
        lines: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        let (project, artwork) = self.bench_artwork()?;
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, Self::bench_settled)?;
        self.bench_left(&mut watch, project, cold)?;
        lines.push(format!(
            "left for the gallery: {}; the display cache {}",
            watch.line(self),
            if cold { "deleted" } else { "kept" }
        ));
        let mut watch = Watch::paced(self);
        self.bench_reopen(&mut watch, project)?;
        let first = watch.ms();
        let filled = |runtime: &Self| runtime.screen.canvas().is_some_and(|c| !c.filling());
        let complete = self.bench_until(&mut watch, filled)?;
        let full = watch.ms();
        let warm = |runtime: &Self| runtime.screen.canvas().is_some_and(|c| !c.warming());
        let warmed = self.bench_until(&mut watch, warm)?;
        let canvas = artwork.canvas();
        lines.push(format!(
            "project {} ({} x {}, {} strokes): the canvas shown after {first:.0} ms, {} {full:.0} \
             ms, its residuals {} {}",
            project.get(),
            canvas.width(),
            canvas.height(),
            strokes(&artwork),
            reached(complete, "complete"),
            reached(warmed, "ready"),
            watch.line(self)
        ));
        Ok(())
    }

    /// The canvas left for the gallery as Gallery does, frames until the gallery shows.
    pub(super) fn bench_leave(&mut self, watch: &mut Watch) -> Result<(), RuntimeError> {
        self.bench_dispatch(vec![Intent::Gallery(GalleryIntent::Show)]);
        let gallery = |runtime: &Self| matches!(runtime.screen, Screen::Gallery(_));
        if self.bench_until(watch, gallery)? {
            Ok(())
        } else {
            Err(RuntimeError::NotNow("the gallery after leaving"))
        }
    }

    /// `project` left for the gallery and its display cache written, deleted after with `cold`:
    /// frames until the cache worker served both (leaving may hand it the canvas to write, so a
    /// delete asked for before the leave could run before that write).
    pub(super) fn bench_left(
        &mut self,
        watch: &mut Watch,
        project: ProjectId,
        cold: bool,
    ) -> Result<(), RuntimeError> {
        self.bench_leave(watch)?;
        if cold {
            self.cache.submit(CacheJob::Delete(project));
        }
        self.bench_until(watch, |runtime| !runtime.cache.busy())?;
        Ok(())
    }

    /// `project` opened from its card in the gallery: frames until its canvas shows.
    pub(super) fn bench_reopen(
        &mut self,
        watch: &mut Watch,
        project: ProjectId,
    ) -> Result<(), RuntimeError> {
        let gallery = (self.screen.gallery_mut()).ok_or(RuntimeError::NotNow("the gallery"))?;
        gallery.dispatch(GalleryIntent::Open(project))?;
        self.bench_until(watch, |runtime| runtime.screen.canvas().is_some())?;
        Ok(())
    }
}
