//! A frame's steps around its UI pass: what the workers finished (a practice sheet left for One
//! more starts the next lesson), the canvas that is ready opening, and the frame's intents
//! applied to the screen.

use openpaint_app::Intent;
use openpaint_practice::Purpose;
use openpaint_practice_app::{OptionsIntent, PracticeIntent, PracticeSheet};

use super::Runtime;
use crate::error::RuntimeError;
use crate::practice::{self, PracticeJob, lesson_job};
use crate::screen::{Screen, dispatch_gallery};
use crate::storage::Job;

impl Runtime {
    /// Applies everything the workers finished (One more's sheet left: the next lesson
    /// starts; Compare's fit or reading to the open sheet), then opens a canvas that is ready.
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
            if let Some(start) = self.screen.answer(done, after) {
                let now = self.frame_clock.wall_seconds();
                let (log, seeds) = (&self.practice, &mut self.seeds);
                match lesson_job(log, (start, Purpose::Day), seeds, now) {
                    Ok(job) => self.storage.submit(job),
                    Err(error) => {
                        log::error!("One more could not start: {error}");
                        self.screen.not_started();
                    }
                }
            }
        }
        match &mut self.screen {
            Screen::Canvas(canvas) => self.compare.receive(canvas),
            _ => self.compare.let_go(),
        }
        self.open_canvas()
    }

    /// The frame's intents applied to the canvas (then practice's on its sheet, its fill, Compare's
    /// fit and copy read off the frame loop, pictures and brush rows) or the gallery (then the
    /// practice options'). Returns whether brush pictures arrived.
    pub(super) fn apply(
        &mut self,
        intents: Vec<Intent>,
        (sheet, options): (Vec<PracticeIntent>, Vec<OptionsIntent>),
    ) -> bool {
        match &mut self.screen {
            Screen::Canvas(canvas) => {
                let now = self.frame_clock.wall_seconds();
                let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
                canvas.dispatch(&self.core, intents, app);
                let day = (&mut self.practice, &mut self.seeds, &mut self.storage);
                practice::apply(canvas, sheet, day, now);
                self.fill.step(canvas);
                self.compare.step(canvas);
                while let Some(item) =
                    practice::sheet(canvas).and_then(PracticeSheet::picture_request)
                {
                    self.storage
                        .submit(Job::Practice(PracticeJob::Picture(item)));
                }
                self.pictures.step(canvas, &mut self.shell);
                self.brushes.step(&self.toolbox, &mut self.shell)
            }
            Screen::Gallery(gallery, asked) => {
                dispatch_gallery(gallery, intents);
                practice::dispatch_options((gallery, asked), options);
                self.pack_pictures.step(&mut self.practice_screens);
                false
            }
            Screen::Opening(_) => false,
        }
    }

    /// Opens a canvas that is ready.
    pub(super) fn open_canvas(&mut self) -> Result<(), RuntimeError> {
        let area = (self.shell.canvas_area(), self.pixels_per_point);
        let app = (&mut self.toolbox, &self.practice);
        self.screen.open(&self.core, area, app)
    }
}
