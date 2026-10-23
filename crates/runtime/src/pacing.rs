//! Frame pacing (DECISIONS D96, D97): no frame is submitted while the GPU is more than one of the
//! screen's frames behind; how long a frame call waits for the frame before depends on what the
//! frame is for (`Wanted`). A frame call that waits is skipped at once: nothing is evaluated,
//! drawn or presented, and the host asks for the next one. Input keeps arriving between frame
//! calls (`Runtime::motion` never touches the GPU) and waits in the runtime's queue, so the next
//! frame drawn takes every sample since: a stroke's samples are never dropped, only drawn a frame
//! or more later. So the main thread never waits for a backlog of GPU work, at most for the rest
//! of the frame before, and Android's input timeout cannot fire however fast the pen moves.

use openpaint_render::Core;

/// Frames the GPU may not have finished when a new one is submitted: the one before.
pub(crate) const MOST_IN_FLIGHT: u64 = 1;

/// Frame calls skipped in a row while the GPU still works on the frame before, at most, while a
/// stroke is drawn; the next is submitted then. On the tablet a frame presented while the GPU
/// worked on the one before waited for it in `present` (Opaque Paint at 400 px: 10 to 21 ms in a
/// stroke's first frames), so a frame heavier than a vsync is waited out by skipping; but a frame
/// drawn after many skips takes all the input since, so its region (and its work) grows with the
/// wait (with no bound, the first stroke over a dense place drew in one frame of 2.6 s): three
/// vsyncs (25 ms at 120 Hz) keep that small.
pub(crate) const PATIENCE: u32 = 3;

/// What a frame call is wanted for, which decides how long it waits for the frame before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wanted {
    /// A stroke is drawn: a frame waits for the one before for up to `PATIENCE` calls, so the
    /// next takes more of the pen's samples and a heavy brush's frame never waits in `present`.
    Stroke,
    /// A hand on or over the glass, or the interface moving (a panel growing, a ring sliding, a
    /// pill fading, the view settling): a frame every vsync. It never waits for the frame
    /// before, only while the GPU has that one and another unfinished: these frames are light,
    /// and their CPU work and GPU work together pass a vsync while the clocks are low, though
    /// each alone is well under it (on the tablet a panel opening drew every third vsync while it
    /// waited, D97).
    Motion,
    /// Nothing on or over the glass and nothing moving: no input to take, so a frame waits until
    /// the frame before was found done a call earlier, and background work queued ahead of the
    /// frames has the GPU (a frame drawn sooner waits for the one before in `present`: Android's
    /// buffer queue takes a frame once the compositor took the one before, at a vsync after the
    /// GPU finished it).
    Rest,
}

impl Wanted {
    /// What a frame call is for: a stroke `drawing`, the screen `resting` (no contact on or over
    /// the glass, no input waiting, no stroke drawn), the interface `moving`.
    pub(crate) const fn of(drawing: bool, (resting, moving): (bool, bool)) -> Self {
        if drawing {
            Self::Stroke
        } else if !resting || moving {
            Self::Motion
        } else {
            Self::Rest
        }
    }
}

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
    /// The call before found the frame before it finished.
    done: bool,
    /// Tests: frame calls still to skip as if the GPU were behind.
    lag: u32,
}

impl Pacing {
    /// Whether this frame call, `wanted` for that, waits for the GPU (`behind`), with the
    /// window's frames in flight (`window`; an offscreen frame is read back, so it never lags);
    /// or a test holds the GPU behind.
    pub(crate) fn skips(&mut self, core: &Core, (window, wanted): (bool, Wanted)) -> bool {
        let in_flight = if window { core.frames_in_flight() } else { 0 };
        self.skips_with(in_flight, (window, wanted))
    }

    /// As `skips`, with `in_flight` of the window's frames unfinished.
    fn skips_with(&mut self, in_flight: u64, (window, wanted): (bool, Wanted)) -> bool {
        let lagging = self.lag > 0;
        self.lag = self.lag.saturating_sub(1);
        let settled = !window || std::mem::replace(&mut self.done, in_flight == 0);
        let skipped = lagging || behind(in_flight, self.waited, (wanted, settled));
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

/// Whether a frame call `wanted` for that waits with `in_flight` of the screen's frames
/// unfinished, `waited` calls in a row having waited already; `settled` when the call before
/// already found the frame before finished (`skips`).
pub(crate) const fn behind(in_flight: u64, waited: u32, (wanted, settled): (Wanted, bool)) -> bool {
    match wanted {
        Wanted::Stroke => {
            in_flight > MOST_IN_FLIGHT || (in_flight == MOST_IN_FLIGHT && waited < PATIENCE)
        }
        Wanted::Motion => in_flight > MOST_IN_FLIGHT,
        Wanted::Rest => in_flight > 0 || !settled,
    }
}

#[cfg(test)]
mod tests {
    use super::{MOST_IN_FLIGHT, PATIENCE, Pacing, Wanted, behind};

    #[test]
    fn a_frame_waits_for_the_one_before_patiently_while_a_stroke_is_drawn_and_wholly_at_rest() {
        let (stroke, rest) = ((Wanted::Stroke, true), (Wanted::Rest, true));
        assert!(
            !behind(0, 0, stroke) && !behind(0, 0, rest),
            "nothing in flight"
        );
        assert!(behind(2, 99, stroke), "two frames behind: always");
        assert!(
            behind(1, PATIENCE - 1, stroke),
            "the frame before, for a while"
        );
        assert!(
            !behind(1, PATIENCE, stroke),
            "then the pen's input goes first"
        );
        assert!(behind(1, 99, rest), "at rest, until it is done");
        assert!(
            behind(0, 0, (Wanted::Rest, false)),
            "and a vsync after it was found done"
        );
        assert!(
            !behind(0, 0, (Wanted::Stroke, false)),
            "but not while a stroke is drawn"
        );
    }

    #[test]
    fn a_hand_or_the_interface_moving_draws_while_the_gpu_has_at_most_the_frame_before() {
        let motion = |settled| (Wanted::Motion, settled);
        assert!(!behind(0, 0, motion(true)) && !behind(0, 0, motion(false)));
        assert!(
            !behind(1, 0, motion(false)),
            "the frame before still on the GPU: drawn at once"
        );
        assert!(behind(2, 0, motion(true)), "two behind: it waits");
        assert!(behind(2, 99, motion(true)), "however long");
    }

    #[test]
    fn what_a_frame_is_for() {
        assert_eq!(Wanted::of(false, (true, false)), Wanted::Rest, "at rest");
        assert_eq!(
            Wanted::of(false, (true, true)),
            Wanted::Motion,
            "a panel growing"
        );
        assert_eq!(
            Wanted::of(false, (false, false)),
            Wanted::Motion,
            "a hand there"
        );
        assert_eq!(
            Wanted::of(true, (false, true)),
            Wanted::Stroke,
            "a stroke, whatever moves"
        );
    }

    /// Frames drawn of `calls` frame calls a vsync apart (as the host asks for one while the
    /// frame before wanted another), each frame's CPU work taking `cpu` vsyncs from its call and
    /// its GPU work `gpu` after that and after the frame before's; the most frames ever
    /// unfinished as one was submitted.
    fn drawn(wanted: Wanted, (cpu, gpu): (f32, f32), calls: u16) -> (u16, usize) {
        let mut pacing = Pacing::default();
        let (mut ends, mut drawn, mut most): (Vec<f32>, u16, usize) = (Vec::new(), 0, 0);
        for call in 0..calls {
            let now = f32::from(call);
            ends.retain(|end| *end > now);
            if !pacing.skips_with(ends.len() as u64, (true, wanted)) {
                most = most.max(ends.len());
                let start = ends.last().map_or(now + cpu, |last| last.max(now + cpu));
                ends.push(start + gpu);
                drawn += 1;
            }
        }
        (drawn, most)
    }

    #[test]
    fn frames_whose_cpu_and_gpu_work_together_pass_a_vsync_come_every_vsync_while_wanted() {
        // As a panel opening on the tablet while the clocks are low: each frame's CPU work and
        // GPU work under a vsync, together over it.
        let light = (0.6, 0.5);
        assert_eq!(drawn(Wanted::Motion, light, 120).0, 120, "every vsync");
        assert_eq!(
            drawn(Wanted::Stroke, light, 120).0,
            60,
            "a stroke: every other"
        );
        assert_eq!(
            drawn(Wanted::Rest, light, 120).0,
            40,
            "at rest: every third"
        );
        // Frames lighter than a vsync together come every vsync however they are wanted.
        let quick = (0.4, 0.3);
        for wanted in [Wanted::Motion, Wanted::Stroke] {
            assert_eq!(drawn(wanted, quick, 120).0, 120, "{wanted:?}");
        }
        // Frames heavier than a vsync on the GPU: never more than the frame before unfinished
        // as one is submitted, and the GPU sets the pace.
        let (heavy, most) = drawn(Wanted::Motion, (0.6, 1.5), 120);
        assert!(most <= MOST_IN_FLIGHT as usize, "{most}");
        assert!((78..=81).contains(&heavy), "the GPU's pace: {heavy}");
    }
}
