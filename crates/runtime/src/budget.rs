//! The GPU memory budget (S49, DECISIONS D76): each frame, before the canvas is evaluated, what its
//! derived caches held is weighed against the budget of the device's class (`Core::memory_budget`).
//! The caches keep memory in this order: the canvas itself (never dropped), the layer pictures,
//! the arena, the snapshots, the view tiers; so above the budget they go the other way round:
//! the view tiers first, then the snapshots (a frame then evaluates every op beneath the stroke),
//! then the arena's blocks the scene does not show, then the layer pictures. Each keeps within
//! the room left to it from then on, so nothing grows back past the budget; nothing fails, and
//! every drop is logged. No pixel changes: each cache is evaluated again from the artwork.

use openpaint_render::{CacheBytes, Core};

use crate::canvas::{Freed, Rooms};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::screen::Screen;

/// The budget, and what the caches held at most.
pub(crate) struct Budget {
    pub(crate) bytes: u64,
    /// The most the caches held, and the most logged.
    peak: u64,
    logged: u64,
    /// The arena's bytes after it was last cut down: it is cut again only once it grew.
    shrunk: Option<u64>,
}

/// The peak is logged again once it grew by this much.
const PEAK_STEP: u64 = 4 << 20;

impl Budget {
    pub(crate) fn of(core: &Core) -> Self {
        let bytes = core.memory_budget();
        log::info!(
            "GPU memory budget {:.0} MB ({:?})",
            mb(bytes),
            core.device_class()
        );
        Self {
            bytes,
            peak: 0,
            logged: 0,
            shrunk: None,
        }
    }

    /// Notes what the caches hold now, logging a new peak.
    fn saw(&mut self, held: &CacheBytes) {
        let total = held.total();
        self.peak = self.peak.max(total);
        if self.peak >= self.logged + PEAK_STEP {
            self.logged = self.peak;
            log::info!(
                "GPU caches hold {:.1} MB (most so far): canvas {:.1}, tiers {:.1}, snapshots {:.1}, \
                 arena {:.1}, pictures {:.1}",
                mb(total),
                mb(held.canvas),
                mb(held.tiers),
                mb(held.snapshots),
                mb(held.arena),
                mb(held.pictures)
            );
        }
    }
}

/// What `budget` leaves each cache when they held `held`, and whether the layer pictures go: the
/// canvas and pictures first, then the arena (cut down when they pass the budget together), the
/// snapshots, the tiers last.
pub(crate) fn rooms_for(held: CacheBytes, budget: u64) -> (Rooms, bool) {
    let kept = held.canvas + held.pictures;
    let snapshots = budget.saturating_sub(kept + held.arena);
    let rooms = Rooms {
        tiers: snapshots.saturating_sub(held.snapshots),
        snapshots,
        shrink_arena: kept + held.arena > budget,
    };
    (rooms, kept > budget)
}

impl Runtime {
    /// The budget's step for this frame on the open canvas; see the module.
    pub(crate) fn keep_within_budget(&mut self) -> Result<(), RuntimeError> {
        let Screen::Canvas(canvas) = &mut self.screen else {
            return Ok(());
        };
        let [width, height] = canvas.target().size();
        let picture = u64::from(width) * u64::from(height) * 4;
        let making = if self.pictures.busy() { picture } else { 0 };
        let held = CacheBytes {
            pictures: making + self.shell.picture_bytes(),
            ..canvas.bytes()
        };
        self.budget.saw(&held);
        let (mut rooms, drop_pictures) = rooms_for(held, self.budget.bytes);
        rooms.shrink_arena &= self.budget.shrunk.is_none_or(|after| held.arena > after);
        let freed = canvas.hold_within(&self.core, rooms)?;
        if rooms.shrink_arena {
            self.budget.shrunk = Some(held.arena.saturating_sub(freed[2]));
        }
        report(freed, &held, self.budget.bytes);
        if drop_pictures && !self.pictures.paused {
            self.pictures.paused = true;
            self.shell.forget_layer_pictures();
            log::warn!(
                "GPU caches over the budget ({:.1} of {:.0} MB): the layer pictures let go",
                mb(held.total()),
                mb(self.budget.bytes)
            );
        } else if self.pictures.paused && held.canvas + held.pictures + picture <= self.budget.bytes
        {
            self.pictures.paused = false;
        }
        Ok(())
    }
}

/// Logs each cache the budget let go.
fn report(freed: Freed, held: &CacheBytes, budget: u64) {
    let names = [
        "the view tiers",
        "the snapshots",
        "the arena's blocks not shown",
    ];
    for (bytes, what) in freed.into_iter().zip(names) {
        if bytes > 0 {
            log::warn!(
                "GPU caches over the budget ({:.1} of {:.0} MB): {what} let go, {:.1} MB",
                mb(held.total()),
                mb(budget),
                mb(bytes)
            );
        }
    }
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1 << 20;

    fn held() -> CacheBytes {
        CacheBytes {
            canvas: 60 * MB,
            tiers: 60 * MB,
            snapshots: 40 * MB,
            arena: 20 * MB,
            pictures: 10 * MB,
        }
    }

    #[test]
    fn above_the_budget_tiers_go_first_then_snapshots_then_the_arena_then_pictures() {
        let all = held().total();
        let (rooms, pictures) = rooms_for(held(), all);
        assert!(rooms.tiers >= 60 * MB && rooms.snapshots >= 40 * MB);
        assert!(
            !rooms.shrink_arena && !pictures,
            "within the budget nothing goes"
        );
        // A little over: only the tiers lose room.
        let (rooms, pictures) = rooms_for(held(), all - 10 * MB);
        assert_eq!(rooms.tiers, 50 * MB);
        assert!(rooms.snapshots >= 40 * MB && !rooms.shrink_arena && !pictures);
        // Past the tiers: the snapshots too.
        let (rooms, pictures) = rooms_for(held(), 110 * MB);
        assert_eq!((rooms.tiers, rooms.snapshots), (0, 20 * MB));
        assert!(!rooms.shrink_arena && !pictures);
        // Past the snapshots: the arena is cut down.
        let (rooms, pictures) = rooms_for(held(), 80 * MB);
        assert_eq!((rooms.tiers, rooms.snapshots), (0, 0));
        assert!(rooms.shrink_arena && !pictures);
        // Past the arena: the pictures go; the canvas never does.
        let (rooms, pictures) = rooms_for(held(), 65 * MB);
        assert!(rooms.shrink_arena && pictures);
    }
}
