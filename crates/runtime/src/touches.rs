//! Whether the screen is idle for background work (S54, S53's option (b)): a frame gives it
//! `BACKGROUND_NS` (4 ms of GPU) while a finger or the pen touches the screen and for
//! `IDLE_AFTER` after the last touch, `IDLE_BACKGROUND_NS` (10 ms) once nothing has touched for
//! that long, so what fills (a fresh start, a view tier) fills about twice as fast while the hand
//! rests, and the first frame of a touch waits for at most one such frame's work.

use std::time::{Duration, Instant};

use openpaint_app::{ContactPhase, InputBatch};

/// How long after the last touch background work keeps the smaller budget.
pub(crate) const IDLE_AFTER: Duration = Duration::from_millis(500);

#[derive(Debug, Default)]
pub(crate) struct Touches {
    /// When a finger or the pen last touched; `None` before the first touch.
    last: Option<Instant>,
}

impl Touches {
    /// Whether the frame taking `batches` finds the screen idle; `drawing` while a stroke is
    /// drawn (a pen held still on the glass sends nothing).
    pub(crate) fn idle(&mut self, batches: &[InputBatch], drawing: bool) -> bool {
        let touched = batches
            .iter()
            .flat_map(|batch| &batch.contacts)
            .any(|contact| !matches!(contact.phase, ContactPhase::Hover | ContactPhase::Leave));
        if touched || drawing {
            self.last = Some(Instant::now());
        }
        self.last.is_none_or(|last| last.elapsed() >= IDLE_AFTER)
    }
}
