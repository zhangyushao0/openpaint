//! The toolbox's brush sets (S47): a committed stroke makes its brush Recent's newest, a carried
//! row lands at another place of its set, the editor's Set row moves the brush to another set's
//! end. None is a history step; each changes what the pickers list and is stored.

use openpaint_core::StrokeTool;

use crate::brush_sets::BrushSet;
use crate::brushes::BrushKey;

use super::{Tool, Toolbox};

impl Toolbox {
    /// A stroke of `tool` was committed (any tool: brush, eraser, smudge): the brush it drew
    /// with is Recent's newest. Recent never repeats a brush and keeps `RECENT_BRUSHES`.
    pub fn drew(&mut self, tool: StrokeTool) {
        let tool = match tool {
            StrokeTool::Paint => Tool::Brush,
            StrokeTool::Erase => Tool::Eraser,
            StrokeTool::Smudge => Tool::Smudge,
        };
        let key = self.brush(tool).key().clone();
        if self.brush_sets.drew(&key) {
            self.changed();
        }
    }

    /// The keys a stored set or Recent listed that the last restore dropped: unknown to the
    /// library, listed twice, or past Recent's length.
    #[must_use]
    pub fn dropped(&self) -> &[String] {
        &self.dropped
    }

    /// `key` lands at place `to` of its set.
    pub(super) fn reorder(&mut self, key: &BrushKey, to: usize) {
        if self.brush_sets.reorder(key, to) {
            self.changed();
        }
    }

    /// `tool`'s brush to the end of `set`.
    pub(super) fn move_to_set(&mut self, tool: Tool, set: BrushSet) {
        let key = self.brush(tool).key().clone();
        if self.brush_sets.move_to(&key, set) {
            self.changed();
        }
    }
}
