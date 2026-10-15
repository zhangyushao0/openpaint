//! The budget's log lines (S49, S51): each new peak of the caches with what the device's
//! allocator holds, what holds memory once a minute while a canvas is open, and every cache the
//! budget let go.

use openpaint_render::{CacheBytes, Core};

use super::ARENA_SHARE;
use crate::canvas::{Canvas, Freed};

/// What holds memory is logged this often while a canvas is open (S51), in seconds.
const NOTE_EVERY: f64 = 60.0;

/// A new peak of the caches, with the allocator's allocations and blocks by label.
pub(super) fn peak(held: &CacheBytes, core: &Core) {
    let gpu = core.gpu_memory().unwrap_or_default();
    let largest: Vec<String> = (gpu.by_label.iter().take(4))
        .map(|(label, bytes)| format!("{label} {:.1}", mb(*bytes)))
        .collect();
    log::info!(
        "GPU caches hold {:.1} MB (most so far): canvas {:.1}, tiers {:.1}, snapshots {:.1}, \
         arena {:.1}, pictures {:.1}; the allocator holds {:.1} MB in blocks of {:.1} ({})",
        mb(held.total()),
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

/// Once every `NOTE_EVERY` seconds of the frame clock, `now` (S51): the strokes, the GPU caches,
/// the device allocator's allocations and blocks with its largest labels, and the heap's scene,
/// plan and history, logged.
pub(super) fn note(
    noted: &mut Option<f64>,
    (core, canvas, held): (&Core, &Canvas, &CacheBytes),
    now: f64,
) {
    if noted.is_some_and(|at| now - at < NOTE_EVERY) {
        return;
    }
    *noted = Some(now);
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
pub(super) fn report(freed: Freed, held: &CacheBytes, budget: u64) {
    let names = [
        "view tiers",
        "the snapshots",
        "the arena's blocks not shown",
    ];
    for (bytes, what) in freed.into_iter().zip(names) {
        if bytes > 0 {
            log::warn!(
                "GPU caches over the budget ({:.1} of {:.0} MB): {what} let go, {:.1} MB, the \
                 least recently used first",
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

pub(super) fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}
