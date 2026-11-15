//! The runner's clock between steps: frames one 60 Hz frame apart for a wait, until motion
//! settles, or up to a moment after the last input; or minutes on at once, the app untouched or
//! hidden (the practice book's time, S89). And the GPU held as the clock is (S129).

use std::time::Instant;

use openpaint_render::Core;
use openpaint_runtime::{Runtime, RuntimeError};

use super::{FRAME_MS, HANG, Runner};
use crate::error::ScenarioError;

/// What the frames do on `core` made independent of how fast its GPU and driver are, as the held
/// clock makes it of how long frames take (S129, D120): the raster core's pipelines made now, not
/// by the raster worker beside the first frames (a view tile a stroke changed before they were
/// made waits to be drawn whole, one after it is drawn in place, a colour step apart in a few
/// pixels), and a frame's budget for background work unbounded, not corrected by the GPU's times
/// as their readings come back (in place or whole again).
pub(super) fn hold_gpu(core: &Core) -> Result<(), ScenarioError> {
    core.make_raster_pipelines().map_err(RuntimeError::Render)?;
    core.set_frame_budget(f64::INFINITY);
    Ok(())
}

impl Runner<'_> {
    /// `frames` frames one 60 Hz frame apart, with no input.
    pub(super) fn wait(&mut self, runtime: &mut Runtime, frames: u32) -> Result<(), ScenarioError> {
        for _ in 0..frames {
            self.frame(runtime)?;
            self.clock_ms += FRAME_MS;
        }
        Ok(())
    }

    /// `minutes` pass with the app in front untouched, or `hidden` (left, as Android's `onPause`,
    /// and come back to); then one frame.
    pub(super) fn later(
        &mut self,
        runtime: &mut Runtime,
        (minutes, hidden): (u32, bool),
    ) -> Result<(), ScenarioError> {
        if hidden {
            runtime.leave();
        }
        self.clock_ms += i64::from(minutes) * 60_000;
        self.frame(runtime)
    }

    /// Frames one 60 Hz frame apart until nothing moves on screen, for at most `HANG`.
    pub(super) fn settle(&mut self, runtime: &mut Runtime) -> Result<(), ScenarioError> {
        let start = Instant::now();
        while runtime.moving() {
            if start.elapsed() > HANG {
                return Err(ScenarioError::Missing("motion did not settle"));
            }
            self.frame(runtime)?;
            self.clock_ms += FRAME_MS;
        }
        Ok(())
    }

    /// Frames one 60 Hz frame apart up to `target`, then one at `target`, and more at `target`
    /// until the canvas has no background work left (`fill_held`).
    pub(super) fn advance_to(
        &mut self,
        runtime: &mut Runtime,
        target: i64,
    ) -> Result<(), ScenarioError> {
        while self.clock_ms < target {
            self.frame(runtime)?;
            self.clock_ms += FRAME_MS;
        }
        let next = self.clock_ms;
        self.clock_ms = target;
        self.frame(runtime)?;
        self.fill_held(runtime)?;
        self.clock_ms = next;
        Ok(())
    }

    /// Frames at the held clock until the canvas's background work is done (its tiles filled
    /// in, the view tier's shown, residuals), for at most `HANG`: what a moment shows of
    /// the canvas never depends on how much of that work the frames before had room for, which
    /// the GPU's measured times decide (`render/src/pace/`): under load or not, s122-path's
    /// dot-inking showed a stroke's tile before or after it was drawn again (S129). Motion holds
    /// still meanwhile: the clock does not move.
    pub(super) fn fill_held(&mut self, runtime: &mut Runtime) -> Result<(), ScenarioError> {
        let start = Instant::now();
        while runtime.canvas_working() {
            if start.elapsed() > HANG {
                return Err(ScenarioError::Missing("the canvas did not fill in"));
            }
            self.frame(runtime)?;
        }
        Ok(())
    }
}
