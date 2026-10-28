//! A frame's steps around its UI pass: what the workers finished (a practice sheet left for One
//! more starts the next session), the canvas that is ready opening, and the frame's intents
//! applied to the screen.

use openpaint_app::Intent;

use super::Runtime;
use crate::error::RuntimeError;
use crate::screen::{Screen, dispatch_gallery};

impl Runtime {
    /// Applies everything the workers finished (One more's sheet left: the next session starts;
    /// Compare's fit or reading to the open sheet), then opens a canvas that is ready.
    pub(crate) fn receive(&mut self) -> Result<(), RuntimeError> {
        self.transfer.receive(&mut self.transfers);
        self.cache.collect();
        while let Some(done) = self.storage.take() {
            let after = (
                &mut self.toolbox,
                &self.core,
                &mut self.cache,
                &mut self.practice,
            );
            if self.screen.answer(done, after) {
                let now = self.frame_clock.wall_seconds();
                let stage = self.practice.stage();
                let job = crate::practice::start_job(
                    &self.practice,
                    (stage, None),
                    &mut self.seeds,
                    now,
                )?;
                self.storage.submit(job);
            }
        }
        match &mut self.screen {
            Screen::Canvas(canvas) => self.compare.receive(canvas),
            _ => self.compare.let_go(),
        }
        self.open_canvas()
    }

    /// The frame's intents applied to the canvas (then its fill, Compare's fit and copy read off
    /// the frame loop, pictures and brush rows) or the gallery. Returns whether brush pictures
    /// arrived.
    pub(super) fn apply(&mut self, intents: Vec<Intent>) -> bool {
        match &mut self.screen {
            Screen::Canvas(canvas) => {
                let now = self.frame_clock.wall_seconds();
                let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds, now);
                canvas.dispatch(&self.core, intents, app);
                self.fill.step(canvas);
                self.compare.step(canvas);
                while let Some(item) = canvas.session_mut().practice_picture_request() {
                    self.storage.submit(crate::storage::Job::Picture(item));
                }
                self.pictures.step(canvas, &mut self.shell);
                self.brushes.step(&self.toolbox, &mut self.shell)
            }
            Screen::Gallery(gallery) => {
                dispatch_gallery(gallery, intents);
                self.pack_pictures.step(&mut self.shell);
                false
            }
            Screen::Opening(_) => false,
        }
    }

    /// Opens a canvas that is ready.
    pub(super) fn open_canvas(&mut self) -> Result<(), RuntimeError> {
        let area = (self.shell.canvas_area(), self.pixels_per_point);
        let app = (&self.toolbox, &self.practice);
        self.screen.open(&self.core, area, app)
    }
}
