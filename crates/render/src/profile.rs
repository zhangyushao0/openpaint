//! What a thread spent since it last asked (S56): the time it waited on the GPU (acquiring and
//! presenting the window's frame, submitting, waiting for submitted work), its longest wait and
//! where, and the time it built scenes. The Android host asks once a frame on the main thread,
//! whose waits block input: a frame that waits seconds lets Android's input timeout kill the app.

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
    /// Evaluating into the screen's canvas (`RegionEvaluator`), its scene builds and waits
    /// included.
    pub evaluated_ns: u64,
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

    use super::Spent;

    /// Nothing spent.
    const NOTHING: Spent = Spent {
        waited_ns: 0,
        longest_ns: 0,
        longest: "",
        scenes: 0,
        scene_ns: 0,
        evaluated_ns: 0,
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
    let ns = nanos(started.elapsed());
    SPENT.with(|spent| {
        let mut now = spent.get();
        now.waited_ns = now.waited_ns.saturating_add(ns);
        if ns >= now.longest_ns {
            now.longest_ns = ns;
            now.longest = what;
        }
        spent.set(now);
    });
    result
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

/// This thread evaluated into a canvas for `took`.
pub(crate) fn evaluated(took: Duration) {
    SPENT.with(|spent| {
        let mut now = spent.get();
        now.evaluated_ns = now.evaluated_ns.saturating_add(nanos(took));
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
        waited("short", || std::thread::sleep(Duration::from_millis(1)));
        let value = waited("long", || {
            std::thread::sleep(Duration::from_millis(5));
            7
        });
        scene_built(Duration::from_millis(3));
        evaluated(Duration::from_millis(4));
        assert_eq!(value, 7);
        let spent = take_spent();
        assert_eq!(spent.longest, "long");
        assert!(spent.longest_ns >= 5_000_000 && spent.waited_ns >= 6_000_000);
        assert_eq!((spent.scenes, spent.scene_ns), (1, 3_000_000));
        assert_eq!(spent.evaluated_ns, 4_000_000);
        assert_eq!(take_spent(), Spent::default(), "taken once");
    }
}
