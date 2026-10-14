//! Frame pacing (S56, DECISIONS D83): no frame is submitted while the GPU is more than one of the
//! screen's frames behind, and while it still works on the frame before, a frame call waits a
//! little first. A frame call that waits is skipped at once: nothing is evaluated, drawn or
//! presented, and the host asks for the next one. Input keeps arriving between frame calls
//! (`Runtime::motion` never touches the GPU) and waits in the runtime's queue, so the next frame
//! drawn takes every sample since: a stroke's samples are never dropped, only drawn a frame or
//! more later. So the main thread never waits for a backlog of GPU work, at most for the rest of
//! the frame before, and Android's input timeout cannot fire however fast the pen moves.

use openpaint_render::Core;

/// Frames the GPU may not have finished when a new one is submitted: the one before.
pub(crate) const MOST_IN_FLIGHT: u64 = 1;

/// Frame calls skipped in a row while the GPU still works on the frame before, at most; the next
/// is submitted then. On the tablet a frame presented while the GPU worked on the one before
/// waited for it in `present` (Opaque Paint at 400 px: 10 to 21 ms in a stroke's first frames,
/// S56), so a frame heavier than a vsync is waited out by skipping; but a frame drawn after many
/// skips takes all the input since, so its region (and its work) grows with the wait (with no
/// bound, the first stroke over a dense place drew in one frame of 2.6 s): three vsyncs (25 ms
/// at 120 Hz) keep that small.
pub(crate) const PATIENCE: u32 = 3;

/// What the last frame call did, for the host's log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Paced {
    /// It was skipped: the GPU was behind.
    pub skipped: bool,
    /// The screen's frames in flight when it began.
    pub in_flight: u64,
}

#[derive(Debug, Default)]
pub(crate) struct Pacing {
    last: Paced,
    /// Frame calls skipped in a row.
    waited: u32,
    /// Tests: frame calls still to skip as if the GPU were behind.
    lag: u32,
}

impl Pacing {
    /// Whether this frame call waits for the GPU: more than `MOST_IN_FLIGHT` of the window's
    /// frames are in flight, or that many and fewer than `PATIENCE` calls waited for them
    /// (`window`; an offscreen frame is read back, so it never lags); or a test holds the GPU
    /// behind.
    pub(crate) fn skips(&mut self, core: &Core, window: bool) -> bool {
        let in_flight = if window { core.frames_in_flight() } else { 0 };
        let lagging = self.lag > 0;
        self.lag = self.lag.saturating_sub(1);
        let behind =
            in_flight > MOST_IN_FLIGHT || (in_flight == MOST_IN_FLIGHT && self.waited < PATIENCE);
        let skipped = lagging || behind;
        self.waited = if skipped { self.waited + 1 } else { 0 };
        self.last = Paced { skipped, in_flight };
        skipped
    }

    pub(crate) const fn last(&self) -> Paced {
        self.last
    }

    pub(crate) fn lag(&mut self, frames: u32) {
        self.lag = frames;
    }
}
