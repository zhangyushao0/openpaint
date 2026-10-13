//! What evaluations did, counted (S53): the op entries dispatched, the plans binned for the
//! screen's canvas and its view tiers and the entries they listed, the smudge residuals evaluated
//! from their strokes, the view tier tiles filled. Read by the harness's `nav-timing`, the app's
//! navigation log and tests: a residual derived again while nothing changed, or a plan binned
//! again every frame, shows as a count.

use std::sync::atomic::{AtomicU64, Ordering};

/// The device's counters, from when it was opened.
#[derive(Debug, Default)]
pub(crate) struct Tally {
    entries: AtomicU64,
    plans: AtomicU64,
    binned: AtomicU64,
    derived: AtomicU64,
    filled: AtomicU64,
}

/// The counters at one moment; `since` gives what happened between two.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Evaluation dispatches submitted.
    pub dispatches: u64,
    /// Their work: op entries listed in their tiles, plus one a tile (`bounded.rs`).
    pub entries: u64,
    /// Plans binned for the screen's canvas and its view tiers, and the entries they listed.
    pub plans: u64,
    pub binned: u64,
    /// Smudge residuals evaluated from their strokes (not taken over from the stroke drawn).
    pub derived: u64,
    /// View tier tiles filled whole.
    pub filled: u64,
}

impl Counts {
    /// What happened from `before` to these.
    #[must_use]
    pub fn since(self, before: Self) -> Self {
        Self {
            dispatches: self.dispatches.saturating_sub(before.dispatches),
            entries: self.entries.saturating_sub(before.entries),
            plans: self.plans.saturating_sub(before.plans),
            binned: self.binned.saturating_sub(before.binned),
            derived: self.derived.saturating_sub(before.derived),
            filled: self.filled.saturating_sub(before.filled),
        }
    }
}

impl Tally {
    pub(crate) fn dispatched(&self, entries: u64) {
        self.entries.fetch_add(entries, Ordering::Relaxed);
    }

    pub(crate) fn binned(&self, entries: usize) {
        self.plans.fetch_add(1, Ordering::Relaxed);
        self.binned.fetch_add(entries as u64, Ordering::Relaxed);
    }

    pub(crate) fn derived(&self) {
        self.derived.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn filled(&self, tiles: usize) {
        self.filled.fetch_add(tiles as u64, Ordering::Relaxed);
    }

    pub(crate) fn counts(&self, dispatches: u64) -> Counts {
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        Counts {
            dispatches,
            entries: load(&self.entries),
            plans: load(&self.plans),
            binned: load(&self.binned),
            derived: load(&self.derived),
            filled: load(&self.filled),
        }
    }
}
