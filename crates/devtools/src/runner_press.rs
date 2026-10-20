//! A press's end: a look just before its lift (S44), or the app interrupted while the contact is
//! still down and the contact cancelled instead of lifted, as Android does (S50).

use openpaint_runtime::{MotionBatch, MotionPhase, Runtime};

use super::Runner;
use crate::error::ScenarioError;
use crate::report::{Record, record};
use crate::scenario::{Interruption, Step};

/// What a press does before its lift: a screenshot and checkpoint, and an interruption.
pub(super) type BeforeLift<'a> = (Option<(&'a String, Option<u64>)>, Option<Interruption>);

impl Runner<'_> {
    /// A press's batches; with `before_lift`, a screenshot of that name before the lift and,
    /// given its hash, a checkpoint of the canvas there too (S44); `interrupted`, the app
    /// interrupted before the lift and the contact cancelled instead (S50).
    pub(super) fn press(
        &mut self,
        runtime: &mut Runtime,
        batches: &[MotionBatch],
        (before_lift, interrupted): BeforeLift<'_>,
    ) -> Result<Vec<Record>, ScenarioError> {
        if before_lift.is_none() && interrupted.is_none() {
            self.gesture(runtime, batches)?;
            return Ok(Vec::new());
        }
        let (lift, held) = batches
            .split_last()
            .ok_or(ScenarioError::Missing("a press has no samples"))?;
        self.gesture(runtime, held)?;
        let mut records = Vec::new();
        if let Some((name, canvas)) = before_lift {
            let shot = Step::Screenshot {
                name: name.clone(),
                after_ms: None,
            };
            records.push(record(runtime, &shot, &self.written)?);
            if let Some(canvas) = canvas {
                let name = name.clone();
                let check = Step::Checkpoint { name, canvas };
                records.push(record(runtime, &check, &self.written)?);
            }
        }
        let Some(interruption) = interrupted else {
            self.gesture(runtime, std::slice::from_ref(lift))?;
            return Ok(records);
        };
        match interruption {
            Interruption::Detach => runtime.detach(),
            Interruption::Leave => runtime.leave(),
        }
        self.frame(runtime)?;
        // Back in front, the platform cancels the contact the window lost.
        let cancel = MotionBatch {
            phase: MotionPhase::Cancel,
            ..lift.clone()
        };
        self.gesture(runtime, &[cancel])?;
        Ok(records)
    }
}
