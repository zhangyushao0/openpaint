//! The runner's clock between steps: frames one 60 Hz frame apart until motion settles, or up to
//! a moment after the last input.

use openpaint_runtime::Runtime;

use super::{FRAME_MS, Runner, SETTLE_FRAMES};
use crate::error::ScenarioError;

impl Runner<'_> {
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
