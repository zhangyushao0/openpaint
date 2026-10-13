//! The GPU memory budget (S49, DECISIONS D76): each frame, before the canvas is evaluated, what its
//! derived caches held is weighed against the budget of the device's class (`Core::memory_budget`).
//! The caches keep memory in this order: the canvas itself (never dropped), the layer pictures,
//! the arena, the snapshots, the view tiers; so above the budget they go the other way round:
//! the view tiers first, then the snapshots (a frame then evaluates every op beneath the stroke),
//! then the arena's blocks the scene does not show, then the layer pictures. Each keeps within
//! the room left to it from then on, so nothing grows back past the budget; nothing fails, and
//! every drop is logged. No pixel changes: each cache is evaluated again from the artwork.
//!
//! S51: the smudge residuals the scene shows keep within a share of the budget whatever else
//! holds memory (`ARENA_SHARE`): beyond it the least recently needed go, never one the last
//! evaluation needed, and an evaluation needing one evaluates it again near what it evaluates.
//! Once a minute while a canvas is open, what holds memory is logged, so a session on the
//! device tells its owners apart.

use std::time::{Duration, Instant};

use openpaint_render::{CacheBytes, Core};

use crate::canvas::{Canvas, Freed, Rooms};
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
    /// When what holds memory was last logged (`note`).
    noted: Option<Instant>,
}

/// The peak is logged again once it grew by this much.
const PEAK_STEP: u64 = 4 << 20;

/// What holds memory is logged this often while a canvas is open (S51).
const NOTE_EVERY: Duration = Duration::from_secs(60);

/// The smudge residuals keep within this share of the budget (S51): 96 MiB on the tablet, 512
/// MiB on a GPU with its own memory. On the tablet the arena binds at most 128 MiB anyway, and
/// a share leaves the snapshots and tiers their room however much is smudged.
pub const ARENA_SHARE: u64 = 4;

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
            noted: None,
        }
    }

    /// Notes what the caches hold now, logging a new peak with what the device's allocator holds
    /// (S51: its live allocations and the memory blocks it took for them).
    fn saw(&mut self, held: &CacheBytes, core: &Core) {
        let total = held.total();
        self.peak = self.peak.max(total);
        if self.peak >= self.logged + PEAK_STEP {
            self.logged = self.peak;
            let gpu = core.gpu_memory().unwrap_or_default();
            let largest: Vec<String> = (gpu.by_label.iter().take(4))
                .map(|(label, bytes)| format!("{label} {:.1}", mb(*bytes)))
                .collect();
            log::info!(
                "GPU caches hold {:.1} MB (most so far): canvas {:.1}, tiers {:.1}, snapshots {:.1}, \
                 arena {:.1}, pictures {:.1}; the allocator holds {:.1} MB in blocks of {:.1} ({})",
                mb(total),
                mb(held.canvas),
                mb(held.tiers),
                mb(held.snapshots),
                mb(held.arena),
                mb(held.pictures),
                mb(gpu.allocated),
                mb(gpu.reserved),
                largest.join(", ")
            );
        }
    }
}

/// What `budget` leaves each cache when they held `held`, and whether the layer pictures go: the
/// canvas and pictures first, then the arena (cut down when they pass the budget together; its
/// residuals within their share), the snapshots, the tiers last.
pub(crate) fn rooms_for(held: CacheBytes, budget: u64) -> (Rooms, bool) {
    let kept = held.canvas + held.pictures;
    let snapshots = budget.saturating_sub(kept + held.arena);
    let rooms = Rooms {
        tiers: snapshots.saturating_sub(held.snapshots),
        snapshots,
        shrink_arena: kept + held.arena > budget,
        arena: budget / ARENA_SHARE,
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
        self.budget.saw(&held, &self.core);
        note(&mut self.budget, &self.core, canvas, &held);
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

/// Once every `NOTE_EVERY` (S51): the strokes, the GPU caches, the device allocator's allocations
/// and blocks with its largest labels, and the heap's scene, plan and history, logged.
fn note(budget: &mut Budget, core: &Core, canvas: &Canvas, held: &CacheBytes) {
    if budget.noted.is_some_and(|at| at.elapsed() < NOTE_EVERY) {
        return;
    }
    budget.noted = Some(Instant::now());
    let artwork = canvas.session().artwork();
    let strokes: usize = (artwork.layers().iter())
        .map(|layer| layer.strokes().len())
        .sum();
    let [scene, plan] = canvas.target().heap_bytes();
    let history = canvas.session().history().total_bytes() as u64;
    let gpu = core.gpu_memory().unwrap_or_default();
    let largest: Vec<String> = (gpu.by_label.iter().take(6))
        .map(|(label, bytes)| format!("{label} {:.1}", mb(*bytes)))
        .collect();
    log::info!(
        "memory at {strokes} strokes: GPU caches {:.1} MB (arena {:.1}); the allocator holds \
         {:.1} MB in blocks of {:.1} ({}); the heap holds the scene's {:.1} MB, the plan's {:.1}, \
         the history's {:.1}",
        mb(held.total()),
        mb(held.arena),
        mb(gpu.allocated),
        mb(gpu.reserved),
        largest.join(", "),
        mb(scene),
        mb(plan),
        mb(history)
    );
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
    if freed[3] > 0 {
        log::info!(
            "smudge residuals past their {:.0} MB: the least recently needed let go, {:.1} MB",
            mb(budget / ARENA_SHARE),
            mb(freed[3])
        );
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
