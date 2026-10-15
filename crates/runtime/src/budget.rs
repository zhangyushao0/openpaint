//! The GPU memory budget (S49, S58, DECISIONS D85): each frame, before the canvas is evaluated,
//! what its derived caches hold is weighed against the budget (`Core::memory_budget`: a quarter
//! of the device's memory on a GPU sharing it, as the host says, 2 GiB on one with its own).
//! Above it, caches go by recency (`to_let_go`), S49's kinds as the units: each view tier, the
//! snapshots, the arena's blocks no scene shows with its material, the layer pictures, the least
//! recently used first, never what the frame shows (the canvas, the tier shown, the pictures
//! while the layer panel is open) or the stroke being drawn uses (the snapshots while a stroke or
//! a preview is drawn, the arena while a smudge is). A cache grows only within the room what is
//! needed now leaves it, so a new tier lets older ones go (`view_kept.rs`). Nothing fails, every
//! drop is logged, no pixel changes: each cache is evaluated again from the artwork.
//!
//! S51: the smudge residuals the scene shows keep within a share of the budget (`ARENA_SHARE`),
//! the least recently needed going first, never one the last evaluation needed. Once a minute
//! while a canvas is open, what holds memory is logged (`budget_note.rs`). S58: `release`, for
//! Android's `onTrimMemory`, lets the tiers, the snapshots and the arena's blocks no scene shows
//! go at once.

use openpaint_render::{CacheBytes, Core};

use crate::canvas::{Canvas, Rooms};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::screen::Screen;

#[path = "budget_note.rs"]
mod note;
use note::{mb, note, report};

/// The budget, and what the caches held at most.
pub(crate) struct Budget {
    pub(crate) bytes: u64,
    /// The most the caches held, and the most logged.
    peak: u64,
    logged: u64,
    /// The arena's bytes after it was last cut down: it is cut again only once it grew.
    shrunk: Option<u64>,
    /// When what holds memory was last logged (`note`).
    noted: Option<f64>,
    /// When the snapshots, the arena and the pictures were last used (the frame clock's seconds).
    used: [f64; 3],
}

/// The smudge residuals keep within this share of the budget (S51).
pub const ARENA_SHARE: u64 = 4;

/// What the budget lets go as one: a view tier (by its scale), the snapshots, the arena's blocks
/// no scene shows with its material, the layer pictures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unit {
    Tier(u32),
    Snapshots,
    Arena,
    Pictures,
}

/// One unit as held now: its bytes, when it was last used, whether the frame needs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Held {
    pub(crate) unit: Unit,
    pub(crate) bytes: u64,
    pub(crate) used: f64,
    pub(crate) needed: bool,
}

/// The units to let go so that `total` bytes held come within `budget`: the least recently used
/// first, never one the frame needs; every one that may go when even that is not enough.
pub(crate) fn to_let_go(held: &[Held], total: u64, budget: u64) -> Vec<Unit> {
    let mut free: Vec<&Held> = (held.iter())
        .filter(|held| !held.needed && held.bytes > 0)
        .collect();
    free.sort_by(|a, b| a.used.total_cmp(&b.used));
    let mut left = total;
    let mut gone = Vec::new();
    for held in free {
        if left <= budget {
            break;
        }
        left = left.saturating_sub(held.bytes);
        gone.push(held.unit);
    }
    gone
}

impl Budget {
    /// The budget of `core`'s device, whose memory is `memory` bytes when its host says; logged.
    pub(crate) fn of(core: &Core, memory: Option<u64>) -> Self {
        let bytes = core.memory_budget(memory);
        let of = memory.map_or_else(
            || "the device's memory not given".to_owned(),
            |memory| format!("the device has {:.0} MB", mb(memory)),
        );
        log::info!(
            "GPU memory budget {:.0} MB ({:?}; {of})",
            mb(bytes),
            core.device_class()
        );
        Self {
            bytes,
            peak: 0,
            logged: 0,
            shrunk: None,
            noted: None,
            used: [0.0; 3],
        }
    }

    /// The units `canvas` and the pictures hold, when each was last used and whether the frame
    /// needs it, the stamps moved on to `now` for what is used now: (drawing or previewing,
    /// smudging, the layer panel open).
    fn units(
        &mut self,
        (canvas, held): (&Canvas, &CacheBytes),
        now: f64,
        using: [bool; 3],
    ) -> Vec<Held> {
        for (used, now_used) in self.used.iter_mut().zip(using) {
            if now_used {
                *used = now;
            }
        }
        let arena = if self.shrunk.is_none_or(|after| held.arena > after) {
            held.arena
        } else {
            0
        };
        let kinds = [
            (Unit::Snapshots, held.snapshots),
            (Unit::Arena, arena),
            (Unit::Pictures, held.pictures),
        ];
        let mut units: Vec<Held> = (kinds.iter().zip(self.used).zip(using))
            .map(|((&(unit, bytes), used), needed)| Held {
                unit,
                bytes,
                used,
                needed,
            })
            .collect();
        units.extend(canvas.view().held().into_iter().map(|tier| Held {
            unit: Unit::Tier(tier.scale),
            bytes: tier.bytes,
            used: tier.shown_at,
            needed: tier.shown,
        }));
        units
    }
}

/// What `budget` leaves each cache to grow into, when `units` are held: what the frame needs of
/// the others taken off; the residuals their share (S51).
fn rooms(held: &CacheBytes, units: &[Held], budget: u64) -> Rooms {
    let needed = |unit: Unit| -> u64 {
        (units.iter())
            .filter(|held| held.needed && held.unit == unit)
            .map(|held| held.bytes)
            .sum()
    };
    let shown = (units.iter())
        .filter(|held| held.needed && matches!(held.unit, Unit::Tier(_)))
        .map(|held| held.bytes)
        .sum::<u64>();
    let fixed = held.canvas + needed(Unit::Pictures);
    Rooms {
        tiers: budget.saturating_sub(fixed + needed(Unit::Snapshots)),
        snapshots: budget.saturating_sub(fixed + shown),
        arena: budget / ARENA_SHARE,
    }
}

impl Runtime {
    /// The budget's step for this frame on the open canvas; see the module.
    pub(crate) fn keep_within_budget(&mut self) -> Result<(), RuntimeError> {
        let now = self.frame_clock.seconds();
        let panel = self.shell.layers_open();
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
        note(&mut self.budget.noted, (&self.core, canvas, &held), now);
        let using = [
            canvas.drawing() || canvas.previewing(),
            canvas.smudging(),
            panel,
        ];
        let units = self.budget.units((canvas, &held), now, using);
        let gone = to_let_go(&units, held.total(), self.budget.bytes);
        let freed =
            canvas.hold_within(&self.core, rooms(&held, &units, self.budget.bytes), &gone)?;
        if gone.contains(&Unit::Arena) {
            self.budget.shrunk = Some(held.arena.saturating_sub(freed[2]));
        }
        report(freed, &held, self.budget.bytes);
        if gone.contains(&Unit::Pictures) && !self.pictures.paused {
            self.pictures.paused = true;
            self.shell.forget_layer_pictures();
            log::warn!(
                "GPU caches over the budget ({:.1} of {:.0} MB): the layer pictures let go",
                mb(held.total()),
                mb(self.budget.bytes)
            );
        } else if self.pictures.paused && held.total() + picture <= self.budget.bytes {
            self.pictures.paused = false;
        }
        Ok(())
    }

    /// The host's word that the app's memory should shrink (Android's `onTrimMemory` at `level`,
    /// S58): from `TRIM_UI_HIDDEN` (the app no longer shows) on, the view tiers, the snapshots
    /// and the arena's blocks no scene shows go at once; they are made again when needed. The
    /// display cache on disk stays. Logged with what went.
    pub fn release(&mut self, level: i32) {
        if level < TRIM_UI_HIDDEN {
            log::info!("trim memory level {level}: nothing let go");
            return;
        }
        let Some(canvas) = self.screen.canvas_mut() else {
            log::info!("trim memory level {level}: no canvas open");
            return;
        };
        let held = canvas.bytes();
        match canvas.release(&self.core) {
            Ok([tiers, snapshots, arena, _]) => log::info!(
                "trim memory level {level}: let go the view tiers {:.1} MB, the snapshots {:.1} \
                 MB, the arena's blocks no scene shows {:.1} MB, of {:.1} MB held; the budget \
                 {:.0} MB",
                mb(tiers),
                mb(snapshots),
                mb(arena),
                mb(held.total()),
                mb(self.budget.bytes)
            ),
            Err(error) => log::error!("trim memory level {level} failed: {error}"),
        }
    }
}

/// Android's `ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN`: the app's interface no longer shows.
pub const TRIM_UI_HIDDEN: i32 = 20;

impl Budget {
    /// Notes what the caches hold now, logging a new peak with what the device's allocator holds
    /// (S51: its live allocations and the memory blocks it took for them).
    fn saw(&mut self, held: &CacheBytes, core: &Core) {
        let total = held.total();
        self.peak = self.peak.max(total);
        if self.peak >= self.logged + PEAK_STEP {
            self.logged = self.peak;
            note::peak(held, core);
        }
    }
}

/// The peak is logged again once it grew by this much.
const PEAK_STEP: u64 = 4 << 20;

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1 << 20;

    fn unit(unit: Unit, bytes: u64, used: f64, needed: bool) -> Held {
        Held {
            unit,
            bytes: bytes * MB,
            used,
            needed,
        }
    }

    #[test]
    fn above_the_budget_the_least_recently_used_go_first_and_the_frames_needs_never() {
        let held = [
            unit(Unit::Tier(4), 60, 9.0, true),
            unit(Unit::Tier(2), 50, 7.0, false),
            unit(Unit::Tier(8), 70, 3.0, false),
            unit(Unit::Snapshots, 40, 5.0, false),
            unit(Unit::Arena, 20, 1.0, false),
            unit(Unit::Pictures, 10, 8.0, false),
        ];
        // The canvas's 100 MB and the units'.
        let total = 100 * MB + 250 * MB;
        assert!(to_let_go(&held, total, total).is_empty(), "within it");
        assert_eq!(to_let_go(&held, total, total - MB), [Unit::Arena]);
        assert_eq!(
            to_let_go(&held, total, total - 21 * MB),
            [Unit::Arena, Unit::Tier(8)]
        );
        assert_eq!(
            to_let_go(&held, total, 200 * MB),
            [Unit::Arena, Unit::Tier(8), Unit::Snapshots, Unit::Tier(2)]
        );
        // However low: the tier shown stays.
        let all = to_let_go(&held, total, 0);
        assert_eq!(all.len(), 5);
        assert!(!all.contains(&Unit::Tier(4)));
        // The stroke being drawn needs the snapshots: they stay, the next oldest go.
        let mut drawing = held;
        drawing[3].needed = true;
        let gone = to_let_go(&drawing, total, 200 * MB);
        assert_eq!(
            gone,
            [Unit::Arena, Unit::Tier(8), Unit::Tier(2), Unit::Pictures]
        );
    }

    #[test]
    fn rooms_leave_each_cache_what_the_needed_others_do_not_take() {
        let held = CacheBytes {
            canvas: 100 * MB,
            tiers: 110 * MB,
            snapshots: 40 * MB,
            arena: 20 * MB,
            pictures: 10 * MB,
        };
        let units = [
            unit(Unit::Tier(4), 60, 9.0, true),
            unit(Unit::Tier(2), 50, 7.0, false),
            unit(Unit::Snapshots, 40, 5.0, true),
            unit(Unit::Pictures, 10, 8.0, false),
        ];
        let rooms = rooms(&held, &units, 1000 * MB);
        assert_eq!(
            rooms.tiers,
            860 * MB,
            "the canvas and the drawn stroke's snapshots"
        );
        assert_eq!(rooms.snapshots, 840 * MB, "the canvas and the tier shown");
        assert_eq!(rooms.arena, 250 * MB);
    }
}
