//! The motion system's one spring and its presets (D44, ARCHITECTURE section 10): a damped
//! spring driven by elapsed time, never per frame, moving towards a target that may change at
//! any moment and keeping its velocity when it does, so motion retargets instead of queueing.
//! Springs are given as Apple gives them: a response (the period of the undamped spring, in
//! seconds) and a damping ratio (1 is critical: no bounce). The values are the ones the author
//! approved in the motion demo (`openpaint-lab/motion-demo`, v3).

use std::f32::consts::TAU;

/// Integration step, in seconds: small enough that the spring moves the same at 60 and 120 Hz.
const STEP: f32 = 1.0 / 960.0;
/// The longest time one frame advances motion, in seconds, so a stall never jumps a motion to
/// its end.
pub const LONGEST_FRAME: f32 = 1.0 / 20.0;

/// A spring's response in seconds and its damping ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feel {
    pub response: f32,
    pub damping: f32,
}

impl Feel {
    fn stiffness(self) -> f32 {
        (TAU / self.response.max(0.01)).powi(2)
    }
}

/// A surface growing out of its source: a panel from its island button, a dialog, a notice.
pub const OPEN: Feel = Feel {
    response: 0.38,
    damping: 1.0,
};
/// A surface going back into its source, faster than it came.
pub const CLOSE: Feel = Feel {
    response: 0.25,
    damping: 1.0,
};
/// In a switch, the old panel retracting into its own button, quickly.
pub const RETRACT: Feel = Feel {
    response: 0.12,
    damping: 1.0,
};
/// Things making room: rows moving to their new place, the view settling at fit.
pub const SETTLE: Feel = Feel {
    response: 0.3,
    damping: 1.0,
};
/// A state changing in place: the ink ring, a switch's knob, a swatch's mark.
pub const ACKNOWLEDGE: Feel = Feel {
    response: 0.18,
    damping: 1.0,
};

/// Seconds after a switch begins that the tapped panel starts to grow.
pub const SWITCH_DELAY: f32 = 0.05;
/// Seconds over which a retracting panel's content fades out in a switch.
pub const RETRACT_FADE: f32 = 0.08;
/// Response of the height relative to the width while opening (below 1 pours down later).
pub const POUR: f32 = 0.92;
/// Growth between which a surface's content fades in.
pub const REVEAL: (f32, f32) = (0.7, 0.97);

/// A unit-mass spring with velocity.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
}

impl Spring {
    /// At rest at `x`.
    #[must_use]
    pub const fn at(x: f32) -> Self {
        Self { x, v: 0.0 }
    }

    /// Advances `dt` seconds towards `target` with `feel`; `slower` (at most 1) stretches the
    /// response. Comes to rest exactly on the target once within a hair of it.
    pub fn step(&mut self, target: f32, feel: Feel, slower: f32, dt: f32) {
        let stiffness = feel.stiffness() * slower * slower;
        let damping = 2.0 * feel.damping * stiffness.sqrt();
        let mut left = dt.clamp(0.0, LONGEST_FRAME);
        while left > 0.0 {
            let h = left.min(STEP);
            let force = stiffness * (target - self.x) - damping * self.v;
            self.v += force * h;
            self.x += self.v * h;
            left -= h;
        }
        if (self.x - target).abs() < 1e-4 && self.v.abs() < 1e-3 {
            *self = Self::at(target);
        }
    }

    #[must_use]
    pub fn at_rest(&self, target: f32) -> bool {
        self.x == target && self.v == 0.0
    }
}

/// 0 below `from`, 1 above `to`, smooth between.
#[must_use]
pub fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `a` to `b` by `t`.
#[must_use]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seconds from 0 until the spring stays within 2 % of 1, and its overshoot.
    fn profile(feel: Feel, frame: f32) -> (f32, f32) {
        let mut spring = Spring::default();
        let (mut settled, mut peak, mut t) = (0.0, 0.0_f32, 0.0);
        while t < 3.0 {
            spring.step(1.0, feel, 1.0, frame);
            t += frame;
            peak = peak.max(spring.x);
            if (spring.x - 1.0).abs() > 0.02 {
                settled = t;
            }
        }
        (settled, (peak - 1.0).max(0.0))
    }

    #[test]
    fn critically_damped_presets_never_overshoot_and_close_faster_than_they_open() {
        let (open, overshoot) = profile(OPEN, 1.0 / 120.0);
        assert!(overshoot < 1e-3, "no bounce: {overshoot}");
        assert!((0.3..=0.4).contains(&open), "opens in about 0.4 s: {open}");
        let (close, _) = profile(CLOSE, 1.0 / 120.0);
        let (retract, _) = profile(RETRACT, 1.0 / 120.0);
        assert!(retract < close && close < open);
    }

    #[test]
    fn the_spring_moves_alike_at_60_and_120_hz() {
        let (at_60, _) = profile(OPEN, 1.0 / 60.0);
        let (at_120, _) = profile(OPEN, 1.0 / 120.0);
        assert!((at_60 - at_120).abs() <= 1.0 / 60.0);
    }

    #[test]
    fn a_reversal_keeps_its_velocity_and_a_stall_never_jumps_to_the_end() {
        let mut spring = Spring::default();
        spring.step(1.0, OPEN, 1.0, 0.1);
        let moving = spring.v;
        assert!(moving > 0.0);
        spring.step(0.0, CLOSE, 1.0, 0.0);
        assert_eq!(spring.v, moving, "retargeting alone changes nothing");
        let mut stalled = Spring::default();
        stalled.step(1.0, OPEN, 1.0, 5.0);
        assert!(
            stalled.x < 0.5,
            "one frame advances at most {LONGEST_FRAME} s"
        );
    }

    #[test]
    fn it_comes_to_rest_exactly_on_its_target() {
        let mut spring = Spring::default();
        for _ in 0..240 {
            spring.step(1.0, SETTLE, 1.0, 1.0 / 120.0);
        }
        assert!(spring.at_rest(1.0));
        assert_eq!(smoothstep(0.7, 0.97, 0.5), 0.0);
        assert_eq!(smoothstep(0.7, 0.97, 1.0), 1.0);
    }
}
