//! Background work at the GPU's pace (S60, DECISIONS D87). While nothing touches the screen (no
//! finger or pen on it or hovering over it for `IDLE_AFTER`, no stroke drawn), every frame call
//! gives the work waiting (the canvas filling, a tier's tiles on either core, the residuals they
//! wait for) what the GPU's queue of background work has room for: a frame drawn does, and so
//! does a call skipped while the GPU is behind, which draws and presents nothing. So that work
//! no longer waits for the frames drawn, and the GPU runs it flat out; the queue holds at most
//! `IDLE_BACKGROUND_NS` of it, which is what the first frame of a touch waits for. The raster
//! core's tiles go to its worker (`raster.rs`), which submits each as the queue has room, so the
//! GPU has the next tile when one ends, between the frame calls too.

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::frame::Runtime;

impl Runtime {
    /// Whether nothing touches the screen now (`touches.rs`).
    pub(crate) fn idle(&mut self) -> bool {
        let drawing = self.screen.canvas().is_some_and(Canvas::drawing);
        self.touches.idle(drawing)
    }

    /// A frame call skipped while the GPU is behind: while nothing touches the screen, the open
    /// canvas's waiting work gets what the GPU's queue has room for, nothing drawn or presented.
    pub(crate) fn ahead(&mut self) -> Result<(), RuntimeError> {
        let waits = (self.screen.canvas())
            .is_some_and(|canvas| canvas.filling() || canvas.warming() || canvas.view_pending());
        if !waits || !self.idle() {
            return Ok(());
        }
        self.core.begin_frame_idle(true);
        self.answer_raster();
        let now = self.frame_clock.seconds();
        if let Some(canvas) = self.screen.canvas_mut() {
            canvas.evaluate(&self.core)?;
            canvas.update_view_again(&self.core, now)?;
        }
        self.core.end_background();
        self.send_raster(true);
        Ok(())
    }

    /// The raster worker's answers go to the open canvas's view tiers.
    pub(crate) fn answer_raster(&mut self) {
        let answers = self.raster.answers();
        if let Some(canvas) = self.screen.canvas_mut() {
            for done in answers {
                canvas.raster_answer(done);
            }
        }
    }

    /// What the open canvas's view tiers ask of the raster worker goes to it; `idle`: while
    /// nothing touches the screen.
    pub(crate) fn send_raster(&mut self, idle: bool) {
        if let Some(canvas) = self.screen.canvas_mut() {
            let orders = canvas.raster_orders();
            self.raster.send(orders, idle);
        }
    }
}
