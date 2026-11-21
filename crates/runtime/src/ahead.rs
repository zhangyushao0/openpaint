//! Background work at the GPU's pace (DECISIONS D96). While nothing touches the screen (no
//! finger or pen on it or hovering over it for `IDLE_AFTER`, no stroke drawn), every frame call
//! gives the work waiting (the canvas filling, a tier's tiles, the residuals they
//! wait for, the snapshots under the next stroke) what the GPU's queue of background work has
//! room for: a frame drawn does, and so does a call skipped while the GPU is behind, which draws
//! and presents nothing. So that work never waits for the frames drawn, and the GPU runs it
//! flat out; the queue holds at most `IDLE_BACKGROUND_NS` of it, which is what the first frame of
//! a touch waits for. The raster core's tiles go to its worker (`raster.rs`), which submits each
//! as the queue has room, so the GPU has the next tile when one ends, between the frame calls
//! too. While a video is made the GPU's spare time is the video's: no frame is idle, a hovering
//! pen takes nothing ahead, and the screen draws only what it shows (the replay stands still).

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::pacing::Wanted;

impl Runtime {
    /// Whether nothing touches the screen now (`touches.rs`).
    pub(crate) fn idle(&mut self) -> bool {
        let drawing = self.screen.canvas().is_some_and(Canvas::drawing);
        self.touches.idle(drawing, self.frame_clock.seconds())
    }

    /// Where the pen hovers (screen pixels) while no stroke is drawn.
    pub(crate) fn hovering(&self) -> Option<[f32; 2]> {
        let drawing = self.screen.canvas().is_some_and(Canvas::drawing);
        self.touches.hovering(drawing)
    }

    /// The frame's budget for background work begun (D96): what the GPU's queue has room for
    /// while nothing touches the screen, else the frame's share, from which the snapshots under
    /// the next stroke take a job while the pen hovers (`Core::pen_hovers`). Whether it is idle.
    pub(crate) fn begin_background(&mut self) -> bool {
        let making = self.making_video();
        let idle = self.idle() && !making;
        self.core.begin_frame_idle(idle);
        if !idle && !making && self.hovering().is_some() {
            self.core.pen_hovers();
        }
        idle
    }

    /// What the frame call now is wanted for (`pacing.rs`): its pacing, and what the recorder
    /// and the bench count.
    pub(crate) fn wanted(&self) -> Wanted {
        let drawing = self.screen.canvas().is_some_and(Canvas::drawing);
        // Input waiting is a hand's (a lift, the last of a drag) even once nothing touches.
        let input = !self.pending.is_empty() || !self.typed.is_empty();
        let resting = self.touches.resting(drawing) && !input;
        let moving = self.shell.animating() || self.replay_moving();
        Wanted::of(drawing, (resting, moving))
    }

    /// A frame call skipped while the GPU is behind: while nothing touches the screen, the open
    /// canvas's waiting work gets what the GPU's queue has room for, nothing drawn or presented.
    pub(crate) fn ahead(&mut self) -> Result<(), RuntimeError> {
        let waits = self.screen.canvas().is_some_and(Canvas::waiting);
        if !waits || !self.idle() || self.making_video() {
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
