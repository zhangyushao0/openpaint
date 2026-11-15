//! What a thread spent since it last asked: the time it waited on the GPU (acquiring and
//! presenting the window's frame, submitting, waiting for submitted work), its longest wait and
//! where, and the time it built scenes. The Android host asks once a frame on the main thread,
//! whose waits block input: a frame that waits seconds lets Android's input timeout kill the app.
//! Beside them, what the stroke being drawn drew onto the canvas and onto the view tier shown, and
//! where each one's drawing went.

use std::time::{Duration, Instant};

/// A thread's GPU waits and scene builds since it last took them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spent {
    /// Every wait, and the longest one and what it was.
    pub waited_ns: u64,
    pub longest_ns: u64,
    pub longest: &'static str,
    /// Scenes built and uploaded (`Scene::build_knowing`, `scene_ops`).
    pub scenes: u32,
    pub scene_ns: u64,
    /// Drawing into the screen's canvas (`CanvasTarget::draw_frame`, and a stroke's frame the
    /// view tier left to it: `stroke_left`), its scene builds and waits included.
    pub drawn_ns: u64,
    /// Drawing the stroke into the view tier shown (`stroke_raster`), its waits included.
    pub tier_ns: u64,
    /// What the stroke's drawing drew onto the canvas and onto the view tier shown.
    pub canvas: StrokeDrew,
    pub tier: StrokeDrew,
}

/// What a stroke's drawing drew onto one output: its regions (a pass chain each), the snapshot
/// parts it took (a tile's snapshot taken, or brought up to the drawing's place), the units
/// (the strokes committed since) its regions laid again over snapshots that lagged them, and
/// where its time went.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StrokeDrew {
    pub regions: u32,
    pub snapshots: u32,
    pub laid_again: u32,
    pub parts: StrokeParts,
}

/// Where a stroke's drawing onto one output went on the frame loop's thread, ns: following the
/// stroke (`track`: its path over every sample so far), the frame's ops (`ops`: the path again,
/// the spans' ops), the plan (`plan`: the kept coverage's slots, the regions, the snapshots
/// settled and their parts, the residuals reached, each region's passes), the passes encoded
/// (`encode`), the buffers written (`writes`: the stroke's ops, the regions' values and
/// instances), the submission (`submit`, also among the waits).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StrokeParts {
    pub track_ns: u64,
    pub ops_ns: u64,
    pub plan_ns: u64,
    pub encode_ns: u64,
    pub writes_ns: u64,
    pub submit_ns: u64,
}

impl StrokeDrew {
    /// Nothing drawn.
    pub const NONE: Self = Self {
        regions: 0,
        snapshots: 0,
        laid_again: 0,
        parts: StrokeParts::NONE,
    };

    /// Both added.
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self {
            regions: self.regions.saturating_add(other.regions),
            snapshots: self.snapshots.saturating_add(other.snapshots),
            laid_again: self.laid_again.saturating_add(other.laid_again),
            parts: self.parts.plus(other.parts),
        }
    }
}

impl StrokeParts {
    /// No time.
    pub const NONE: Self = Self {
        track_ns: 0,
        ops_ns: 0,
        plan_ns: 0,
        encode_ns: 0,
        writes_ns: 0,
        submit_ns: 0,
    };

    /// Both added.
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self {
            track_ns: self.track_ns.saturating_add(other.track_ns),
            ops_ns: self.ops_ns.saturating_add(other.ops_ns),
            plan_ns: self.plan_ns.saturating_add(other.plan_ns),
            encode_ns: self.encode_ns.saturating_add(other.encode_ns),
            writes_ns: self.writes_ns.saturating_add(other.writes_ns),
            submit_ns: self.submit_ns.saturating_add(other.submit_ns),
        }
    }
}

/// The nanoseconds since `started`.
pub(crate) fn since(started: Instant) -> u64 {
    nanos(started.elapsed())
}

#[cfg_attr(
    target_os = "android",
    expect(
        clippy::missing_const_for_thread_local,
        reason = "clippy 1.98 takes every const thread-local for a non-const one on Android"
    )
)]
mod cell {
    use std::cell::Cell;

    use super::{Spent, StrokeDrew};

    /// Nothing spent.
    const NOTHING: Spent = Spent {
        waited_ns: 0,
        longest_ns: 0,
        longest: "",
        scenes: 0,
        scene_ns: 0,
        drawn_ns: 0,
        tier_ns: 0,
        canvas: StrokeDrew::NONE,
        tier: StrokeDrew::NONE,
    };

    thread_local! {
        pub(super) static SPENT: Cell<Spent> = const { Cell::new(NOTHING) };
    }
}
use cell::SPENT;

/// Calls `call`, a GPU call that may block (`what` names it), and counts the time it took as a
/// wait of this thread.
pub(crate) fn waited<T>(what: &'static str, call: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let result = call();
    wait_took(what, started.elapsed());
    result
}

/// This thread waited `took` in `what`.
fn wait_took(what: &'static str, took: Duration) {
    let ns = nanos(took);
    SPENT.with(|spent| {
        let mut now = spent.get();
        now.waited_ns = now.waited_ns.saturating_add(ns);
        if ns >= now.longest_ns {
            now.longest_ns = ns;
            now.longest = what;
        }
        spent.set(now);
    });
}

/// This thread built and uploaded a scene in `took`.
pub(crate) fn scene_built(took: Duration) {
    SPENT.with(|spent| {
        let mut now = spent.get();
        now.scenes += 1;
        now.scene_ns = now.scene_ns.saturating_add(nanos(took));
        spent.set(now);
    });
}

/// This thread drew into a canvas for `took`.
pub(crate) fn drawn(took: Duration) {
    SPENT.with(|spent| {
        let mut now = spent.get();
        now.drawn_ns = now.drawn_ns.saturating_add(nanos(took));
        spent.set(now);
    });
}

/// The stroke's drawing drew `drew` onto the canvas.
pub(crate) fn canvas_drew(drew: StrokeDrew) {
    SPENT.with(|spent| {
        let mut now = spent.get();
        now.canvas = now.canvas.plus(drew);
        spent.set(now);
    });
}

/// The stroke's drawing drew `drew` onto the view tier shown, in `took`.
pub(crate) fn tier_drew(drew: StrokeDrew, took: Duration) {
    SPENT.with(|spent| {
        let mut now = spent.get();
        now.tier = now.tier.plus(drew);
        now.tier_ns = now.tier_ns.saturating_add(nanos(took));
        spent.set(now);
    });
}

/// What this thread spent since the last call, and from now on nothing.
#[must_use]
pub fn take_spent() -> Spent {
    SPENT.with(|spent| spent.replace(Spent::default()))
}

fn nanos(took: Duration) -> u64 {
    u64::try_from(took.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_add_up_and_the_longest_is_named() {
        let _ = take_spent();
        let ms = Duration::from_millis;
        wait_took("short", ms(1));
        wait_took("long", ms(5));
        wait_took("shorter", ms(2));
        scene_built(ms(3));
        drawn(ms(4));
        let drew = |regions, snapshots, laid_again| StrokeDrew {
            regions,
            snapshots,
            laid_again,
            parts: StrokeParts {
                plan_ns: u64::from(regions),
                ..StrokeParts::NONE
            },
        };
        canvas_drew(drew(1, 0, 0));
        tier_drew(drew(2, 1, 3), ms(6));
        tier_drew(drew(3, 0, 2), ms(1));
        let spent = take_spent();
        assert_eq!(spent.longest, "long");
        assert_eq!((spent.longest_ns, spent.waited_ns), (5_000_000, 8_000_000));
        assert_eq!((spent.scenes, spent.scene_ns), (1, 3_000_000));
        assert_eq!(spent.drawn_ns, 4_000_000);
        assert_eq!((spent.canvas, spent.tier), (drew(1, 0, 0), drew(5, 1, 5)));
        assert_eq!(spent.tier_ns, 7_000_000);
        assert_eq!(take_spent(), Spent::default(), "taken once");
    }

    /// A wait counts the time its call took: at least what a sleep asked for, however loaded
    /// the machine (a sleep is never compared with another, whose timer step and scheduling
    /// vary: on Windows a 1-ms sleep once outlasted a 5-ms one).
    #[test]
    fn a_wait_counts_the_time_its_call_took() {
        let _ = take_spent();
        let value = waited("sleep", || {
            std::thread::sleep(Duration::from_millis(2));
            7
        });
        assert_eq!(value, 7);
        let spent = take_spent();
        assert_eq!(spent.longest, "sleep");
        assert!(spent.waited_ns >= 2_000_000, "{spent:?}");
        assert_eq!(spent.longest_ns, spent.waited_ns);
    }
}
