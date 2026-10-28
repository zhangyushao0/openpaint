//! The runner's clock between steps: frames one 60 Hz frame apart for a wait, until motion
//! settles, or up to a moment after the last input; or minutes on at once, the app untouched or
//! hidden (the practice book's time, S89).

use openpaint_runtime::Runtime;

use super::{FRAME_MS, Runner, SETTLE_FRAMES};
use crate::error::ScenarioError;

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

    /// Frames one 60 Hz frame apart until nothing moves on screen, at most `SETTLE_FRAMES`.
    pub(super) fn settle(&mut self, runtime: &mut Runtime) -> Result<(), ScenarioError> {
        for _ in 0..SETTLE_FRAMES {
            if !runtime.moving() {
                return Ok(());
            }
            self.frame(runtime)?;
            self.clock_ms += FRAME_MS;
        }
        Err(ScenarioError::Missing("motion did not settle"))
    }

    /// Frames one 60 Hz frame apart up to `target`, then one at `target`.
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
        self.clock_ms = next;
        Ok(())
    }
}
