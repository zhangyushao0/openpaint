//! What drawing did, counted: the smudge residuals derived from their strokes, the view
//! tier tiles filled, the regions the raster core drew, the snapshots taken under a stroke and the
//! scenes that took a lift's strokes in place. Read by the app's
//! navigation log, its benchmarks and tests: a residual derived again while nothing changed, or a
//! tile drawn again every frame, shows as a count.

use std::sync::atomic::{AtomicU64, Ordering};

/// The device's counters, from when it was opened.
#[derive(Debug, Default)]
pub(crate) struct Tally {
    derived: AtomicU64,
    filled: AtomicU64,
    rastered: AtomicU64,
    snapshots: AtomicU64,
    appended: AtomicU64,
}

/// The counters at one moment; `since` gives what happened between two.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Smudge residuals derived from their strokes (not taken over from the stroke drawn).
    pub derived: u64,
    /// View tier tiles filled whole.
    pub filled: u64,
    /// Regions the raster core drew: canvas and view tier tiles and their changed parts,
    /// export pieces, materials, a stroke's frames.
    pub rastered: u64,
    /// Tiles of a stroke's drawing whose snapshot was taken (the units under the stroke drawn).
    pub snapshots: u64,
    /// Scenes that took the strokes a version appends on their place in place, not built.
    pub appended: u64,
}

impl Counts {
    /// What happened from `before` to these.
    #[must_use]
    pub fn since(self, before: Self) -> Self {
        Self {
            derived: self.derived.saturating_sub(before.derived),
            filled: self.filled.saturating_sub(before.filled),
            rastered: self.rastered.saturating_sub(before.rastered),
            snapshots: self.snapshots.saturating_sub(before.snapshots),
            appended: self.appended.saturating_sub(before.appended),
        }
    }
}

impl Tally {
    pub(crate) fn derived(&self) {
        self.derived.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn filled(&self, tiles: usize) {
        self.filled.fetch_add(tiles as u64, Ordering::Relaxed);
    }

    pub(crate) fn rastered(&self, regions: usize) {
        self.rastered.fetch_add(regions as u64, Ordering::Relaxed);
    }

    pub(crate) fn appended(&self) {
        self.appended.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) {
        self.snapshots.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn counts(&self) -> Counts {
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        Counts {
            derived: load(&self.derived),
            filled: load(&self.filled),
            rastered: load(&self.rastered),
            snapshots: load(&self.snapshots),
            appended: load(&self.appended),
        }
    }
}
