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
    /// The call before found the frame before it finished (S60).
    done: bool,
    /// Tests: frame calls still to skip as if the GPU were behind.
    lag: u32,
}

impl Pacing {
    /// Whether this frame call waits for the GPU: more than `MOST_IN_FLIGHT` of the window's
    /// frames are in flight, or that many and fewer than `PATIENCE` calls waited for them; or,
    /// `resting` (no contact on the glass or hovering, no stroke drawn, S60), any, or the frame
    /// before was found finished only now (`window`; an offscreen frame is read back, so it never
    /// lags); or a test holds the GPU behind. With no input to take a frame has no reason to go
    /// before the one before is shown: the GPU runs background work queued ahead of the frames
    /// then, and a frame drawn sooner waits for the one before in `present` (Android's buffer
    /// queue takes a frame once the compositor took the one before, at a vsync after the GPU
    /// finished it).
    pub(crate) fn skips(&mut self, core: &Core, (window, resting): (bool, bool)) -> bool {
        let in_flight = if window { core.frames_in_flight() } else { 0 };
        let lagging = self.lag > 0;
        self.lag = self.lag.saturating_sub(1);
        let settled = !window || std::mem::replace(&mut self.done, in_flight == 0);
        let skipped = lagging || behind(in_flight, self.waited, (resting, settled));
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

/// Whether a frame call waits with `in_flight` of the screen's frames unfinished, `waited` calls
/// in a row having waited already; `resting` when no hand is on or over the glass, `settled` when
/// the call before already found the frame before finished (`skips`).
pub(crate) const fn behind(in_flight: u64, waited: u32, (resting, settled): (bool, bool)) -> bool {
    let patient = resting || waited < PATIENCE;
    in_flight > MOST_IN_FLIGHT || (in_flight == MOST_IN_FLIGHT && patient) || (resting && !settled)
}

#[cfg(test)]
mod tests {
    use super::{PATIENCE, behind};

    #[test]
    fn a_frame_waits_for_the_one_before_patiently_while_a_hand_is_there_and_wholly_while_none() {
        let (hand, rest) = ((false, true), (true, true));
        assert!(
            !behind(0, 0, hand) && !behind(0, 0, rest),
            "nothing in flight"
        );
        assert!(behind(2, 99, hand), "two frames behind: always");
        assert!(
            behind(1, PATIENCE - 1, hand),
            "the frame before, for a while"
        );
        assert!(!behind(1, PATIENCE, hand), "then a hand's input goes first");
        assert!(behind(1, 99, rest), "with no hand there, until it is done");
        assert!(
            behind(0, 0, (true, false)),
            "and a vsync after it was found done"
        );
        assert!(!behind(0, 0, (false, false)), "but not with a hand there");
    }
}
