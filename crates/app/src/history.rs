//! History: the artwork versions the user can step back and forth through (DECISIONS D08:
//! undo is a version switch). Recording a version drops the ones undone before it; the oldest
//! steps are trimmed once the steps kept hold more than the byte budget.

use std::collections::VecDeque;

use openpaint_core::Artwork;

/// Bytes the kept steps may hold beyond the oldest version: about 2.8 million pen samples,
/// hours of drawing, a small part of a tablet's memory.
pub const HISTORY_BUDGET: usize = 64 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryIntent {
    /// Back to the version before the current one.
    Undo,
    /// Forward again to the version undone last.
    Redo,
}

#[derive(Debug)]
struct Entry {
    artwork: Artwork,
    /// What this version holds that the entry before it does not share; 0 for the oldest.
    bytes: usize,
}

/// Versions oldest first, the current one at `cursor`; those after it can be redone.
#[derive(Debug)]
pub struct History {
    entries: VecDeque<Entry>,
    cursor: usize,
    /// The sum of every entry's `bytes`.
    total_bytes: usize,
    budget: usize,
}

impl History {
    /// A history whose only version is `artwork`, keeping steps up to `budget` bytes.
    #[must_use]
    pub fn new(artwork: Artwork, budget: usize) -> Self {
        Self {
            entries: VecDeque::from([Entry { artwork, bytes: 0 }]),
            cursor: 0,
            total_bytes: 0,
            budget,
        }
    }

    /// The version the user sees.
    #[must_use]
    pub fn current(&self) -> &Artwork {
        &self.entries[self.cursor].artwork
    }

    /// Makes `artwork` the current version. The versions undone before it can no longer be
    /// redone; the oldest steps go while the kept ones exceed the budget, the newest step
    /// always stays. Returns false, changing nothing, when `artwork` already is the current
    /// version.
    pub fn record(&mut self, artwork: Artwork) -> bool {
        if artwork.same_version(self.current()) {
            return false;
        }
        for undone in self.entries.drain(self.cursor + 1..) {
            self.total_bytes -= undone.bytes;
        }
        let bytes = artwork.bytes_since(self.current());
        self.entries.push_back(Entry { artwork, bytes });
        self.cursor += 1;
        self.total_bytes += bytes;
        self.trim();
        true
    }

    /// Steps back one version; `None` when there is none.
    pub fn undo(&mut self) -> Option<&Artwork> {
        self.cursor = self.cursor.checked_sub(1)?;
        Some(self.current())
    }

    /// Steps forward to the version undone last; `None` when there is none.
    pub fn redo(&mut self) -> Option<&Artwork> {
        if !self.can_redo() {
            return None;
        }
        self.cursor += 1;
        Some(self.current())
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.entries.len()
    }

    /// How many undos in a row are possible.
    #[must_use]
    pub fn undo_steps(&self) -> usize {
        self.cursor
    }

    /// How many redos in a row are possible.
    #[must_use]
    pub fn redo_steps(&self) -> usize {
        self.entries.len() - 1 - self.cursor
    }

    /// Bytes held by the kept steps; above the budget only when the newest step alone is.
    #[must_use]
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Drops the oldest version while the steps exceed the budget. Called right after a
    /// version is recorded; the step to it always stays, so the last edit can be undone even
    /// when it alone exceeds the budget.
    fn trim(&mut self) {
        while self.total_bytes > self.budget && self.cursor > 1 {
            self.entries.pop_front();
            self.cursor -= 1;
            if let Some(oldest) = self.entries.front_mut() {
                self.total_bytes -= oldest.bytes;
                oldest.bytes = 0;
            }
        }
    }
}
