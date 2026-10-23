//! A motion of the interface measured as the display shows it (`bench ui`): a hand's gesture fed
//! in real time, a frame call at each vsync a frame is wanted at (as the host asks: after input,
//! and again while the frame before wanted the next), until the gesture is over and nothing on
//! screen moves. Each vsync from the gesture's first input to the motion's last frame is drawn,
//! skipped by the pacing (a frame of the screen still in flight, or none: the frame before was
//! found done only now), missed (the frame call before was still working), or not wanted. A
//! motion's line: the frames drawn against the vsyncs wanting one, the longest gap (from a frame
//! wanted to its frame drawn, in vsyncs: 1 is every vsync), the calls skipped and why, each drawn
//! frame's work and UI pass, its GPU time by timestamps, and the waits for the GPU.

use super::hand::batch;
use super::{VSYNC, Watch, spread};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::{MotionPhase, ToolKind};

/// A finger or the pen on the glass for a tap, ms.
const TAP_MS: i64 = 64;
/// Between a gesture's batches, ms.
const STEP_MS: i64 = 8;
/// How long a motion may go on after its last input before it is cut, ms.
const TAIL_MS: f64 = 2000.0;

/// One input batch of a gesture: when (ms from the motion's start), what, by what, where.
#[derive(Debug, Clone, Copy)]
pub(super) struct Step {
    ms: i64,
    phase: MotionPhase,
    tool: ToolKind,
    at: [f32; 2],
}

pub(super) type Gesture = Vec<Step>;

/// A tap at `at` (screen pixels) by `tool`.
pub(super) fn tap(at: [f32; 2], tool: ToolKind) -> Gesture {
    let step = |ms, phase| Step {
        ms,
        phase,
        tool,
        at,
    };
    vec![step(0, MotionPhase::Down), step(TAP_MS, MotionPhase::Up)]
}

/// A finger down at `path`'s first point, moved straight to each next over `ms` in all (a batch
/// each `STEP_MS`), lifted at the last.
pub(super) fn drag(path: &[[f32; 2]], ms: i64) -> Gesture {
    let legs = path.len().saturating_sub(1).max(1);
    let steps = (ms / STEP_MS).max(legs as i64);
    let at = |k: i64| {
        let along = k as f32 / steps as f32 * legs as f32;
        let leg = (along as usize).min(legs - 1);
        let (from, to) = (path[leg], path[(leg + 1).min(path.len() - 1)]);
        let t = along - leg as f32;
        [0, 1].map(|axis| from[axis] + (to[axis] - from[axis]) * t)
    };
    let step = |k: i64, phase| Step {
        ms: k * STEP_MS,
        phase,
        tool: ToolKind::Finger,
        at: at(k),
    };
    let mut gesture = vec![step(0, MotionPhase::Down)];
    gesture.extend((1..steps).map(|k| step(k, MotionPhase::Move)));
    gesture.push(step(steps, MotionPhase::Up));
    gesture
}

/// `gesture` by the pen, which then hovers where it lifted for `ms` and goes.
pub(super) fn hovering(mut gesture: Gesture, ms: i64) -> Gesture {
    let Some(last) = gesture.last().copied() else {
        return gesture;
    };
    let hover = |k: i64, phase| Step {
        ms: last.ms + k * STEP_MS,
        phase,
        ..last
    };
    let steps = ms / STEP_MS;
    gesture.extend((1..steps).map(|k| hover(k, MotionPhase::HoverMove)));
    gesture.push(hover(steps, MotionPhase::HoverExit));
    gesture
}

/// What a vsync of a motion did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vsync {
    Drawn,
    /// Skipped by the pacing, with the screen's frames in flight then.
    Skipped(u64),
    /// Wanted, but the frame call before was still working.
    Missed,
    Idle,
}

/// A measured motion.
#[derive(Debug, Default)]
pub(super) struct Motion {
    /// From the gesture's first input on.
    vsyncs: Vec<Vsync>,
    /// The vsyncs up to the last frame drawn while the hand was there or something moved after
    /// it: the motion ends with the next frame drawn, which shows it at rest.
    moved: usize,
    /// Drawn frames' work and UI pass, and their GPU time, ms.
    work: Vec<f64>,
    ui: Vec<f64>,
    gpu: Vec<f64>,
    /// Waits for the GPU, ms, and the longest with what it was.
    waited: f64,
    longest: (f64, &'static str),
}

impl Motion {
    /// "<drawn> of <wanted> vsyncs (<share> %), longest gap <n>; ..." (see the module).
    pub(super) fn line(&self) -> String {
        let drawn_at = |at: &usize| self.vsyncs[*at] == Vsync::Drawn;
        let end = (self.moved..self.vsyncs.len())
            .find(drawn_at)
            .or_else(|| (0..self.vsyncs.len()).rfind(drawn_at));
        let shown = &self.vsyncs[..end.map_or(0, |end| end + 1)];
        let wanted = shown.iter().filter(|v| **v != Vsync::Idle).count();
        let drawn = shown.iter().filter(|v| **v == Vsync::Drawn).count();
        let (mut gap, mut waiting) = (0, 0);
        for vsync in shown {
            match vsync {
                Vsync::Drawn => {
                    gap = gap.max(waiting + 1);
                    waiting = 0;
                }
                Vsync::Skipped(_) | Vsync::Missed => waiting += 1,
                Vsync::Idle => {}
            }
        }
        let count = |kind: fn(&Vsync) -> bool| shown.iter().filter(|v| kind(v)).count();
        let behind = count(|v| matches!(v, Vsync::Skipped(n) if *n > 0));
        let resting = count(|v| *v == Vsync::Skipped(0));
        let missed = count(|v| *v == Vsync::Missed);
        let share = 100.0 * drawn as f64 / wanted.max(1) as f64;
        format!(
            "{drawn} of {wanted} vsyncs ({share:.0} %), longest gap {gap}; skipped {} ({behind} \
             a frame in flight, {resting} none), missed {missed}; frame work {} (UI pass {}), GPU \
             a frame {}; waits {:.1} (longest {:.1} in {})",
            behind + resting,
            spread(&self.work),
            spread(&self.ui),
            spread(&self.gpu),
            self.waited,
            self.longest.0,
            if self.longest.1.is_empty() {
                "-"
            } else {
                self.longest.1
            }
        )
    }
}

impl Runtime {
    /// `gesture` fed as a hand makes it, frames as the display asks for them, until it is over
    /// and nothing moves (at most `TAIL_MS` after its last input): the motion measured.
    pub(super) fn bench_motion(&mut self, gesture: Gesture) -> Result<Motion, RuntimeError> {
        self.core.wait_idle()?;
        let _ = self.core.frame_times();
        self.core.time_frames(true);
        let last = gesture.last().map_or(0, |step| step.ms) as f64;
        let base = self.clock.now_ms() + 50;
        let mut steps = gesture.into_iter().peekable();
        let mut motion = Motion::default();
        // Always at the display's cadence: vsyncs are what this measures.
        let mut watch = Watch {
            paced: true,
            ..Watch::start(self)
        };
        let vsync = VSYNC.as_secs_f64() * 1e3;
        let (mut again, mut still) = (false, 0);
        loop {
            let now = watch.ms();
            let mut fed = false;
            while let Some(step) = steps.next_if(|step| step.ms as f64 <= now) {
                let at = std::slice::from_ref(&step.at);
                self.motion(&batch(step.phase, step.tool, at, base + step.ms));
                fed = true;
            }
            let wanted = fed || again;
            if fed {
                still = 0;
            }
            // The vsyncs since the last call: wanted (and missed) while a frame still is.
            let tick = (now / vsync) as usize;
            let before = if wanted { Vsync::Missed } else { Vsync::Idle };
            let first = motion.vsyncs.len();
            motion.vsyncs.extend((first..tick).map(|_| before));
            if !wanted {
                if steps.peek().is_none() {
                    break;
                }
                motion.vsyncs.push(Vsync::Idle);
                watch.wait(false);
                continue;
            }
            again = self.bench_call(&mut watch)?;
            let over = steps.peek().is_none();
            if self.bench_took(&watch, &mut motion) {
                // A tap's intent shows, and may start moving, a frame after the one taking it.
                let moving = self.shell.animating();
                still = if moving { 0 } else { still + 1 };
                if moving || !over {
                    motion.moved = motion.vsyncs.len();
                }
            }
            if (over && still >= 2) || now > last + TAIL_MS {
                break;
            }
            watch.wait(false);
        }
        self.core.wait_idle()?;
        motion
            .gpu
            .extend(self.core.frame_times().iter().map(|ns| ns / 1e6));
        self.core.time_frames(false);
        Ok(motion)
    }

    /// What the frame call just made did, into `motion`; whether it drew.
    fn bench_took(&self, watch: &Watch, motion: &mut Motion) -> bool {
        let paced = self.paced();
        let ms = |ns: u64| ns as f64 / 1e6;
        motion
            .gpu
            .extend(self.core.frame_times().iter().map(|ns| ns / 1e6));
        if paced.skipped {
            motion.vsyncs.push(Vsync::Skipped(paced.in_flight));
            return false;
        }
        motion.vsyncs.push(Vsync::Drawn);
        motion.work.extend(watch.frames.last().copied());
        motion.ui.push(ms(self.ui_ns));
        let spent = watch.last;
        motion.waited += ms(spent.waited_ns);
        if ms(spent.longest_ns) > motion.longest.0 {
            motion.longest = (ms(spent.longest_ns), spent.longest);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_motion_counts_the_vsyncs_wanting_a_frame_and_its_longest_gap() {
        let motion = Motion {
            vsyncs: vec![
                Vsync::Drawn,
                Vsync::Idle,
                Vsync::Idle,
                Vsync::Drawn,
                Vsync::Skipped(1),
                Vsync::Skipped(0),
                Vsync::Drawn,
                Vsync::Missed,
                Vsync::Drawn,
                Vsync::Skipped(1),
                Vsync::Skipped(0),
                Vsync::Drawn,
            ],
            // The third frame drawn still moved: the fourth shows the motion at rest.
            moved: 7,
            ..Motion::default()
        };
        let line = motion.line();
        assert!(
            line.starts_with("4 of 7 vsyncs (57 %), longest gap 3;"),
            "{line}"
        );
        assert!(
            line.contains("skipped 2 (1 a frame in flight, 1 none), missed 1"),
            "{line}"
        );
    }

    #[test]
    fn gestures_feed_a_batch_each_step_and_the_pen_hovers_after_its_tap() {
        let dragged = drag(&[[0.0, 0.0], [80.0, 0.0]], 80);
        assert_eq!(dragged.len(), 11);
        assert_eq!(dragged[5].at, [40.0, 0.0]);
        assert_eq!(dragged[10].phase, MotionPhase::Up);
        let pen = hovering(tap([5.0, 6.0], ToolKind::Stylus), 40);
        assert_eq!(pen.len(), 2 + 5);
        assert_eq!(pen[2].phase, MotionPhase::HoverMove);
        assert_eq!(
            (pen[6].phase, pen[6].ms),
            (MotionPhase::HoverExit, TAP_MS + 40)
        );
    }
}
