//! The toolbox's brush intents: choosing a brush and setting its size, opacity and StreamLine
//! for the tool the pen uses now, and since S46 the brush editor's: a setting of the brush,
//! its name, a copy, Reset and Delete. Editing a brush changes it for every tool from the next
//! stroke on; strokes already drawn keep their recipes, and nothing is a history step.

use crate::brush_edit::{BrushChange, edited};
use crate::brushes::{BrushKey, SizeRange};
use crate::tool_intent::BrushIntent;

use super::{MIN_OPACITY, OPACITY, SIZE, STREAMLINE, Tool, Toolbox, rows_of};

impl Toolbox {
    /// Applies `intent` to the tool the pen uses now (the editor's to its brush). Choosing a
    /// brush the library does not hold, or a value that is not a number, changes nothing.
    pub fn dispatch(&mut self, intent: &BrushIntent) {
        let tool = self.current();
        match intent {
            BrushIntent::Choose(key) => {
                if self.library.brush(key).is_some() {
                    let set = &mut self.sets[tool.index()];
                    self.unsaved |= *key != set.current;
                    set.current = key.clone();
                    self.set_picking(false);
                }
            }
            &BrushIntent::Size { position, settled } if position.is_finite() => {
                let size = self.brush(tool).size().at(position);
                self.set(SIZE, size, settled);
            }
            &BrushIntent::Opacity { opacity, settled } if opacity.is_finite() => {
                self.set(OPACITY, opacity.clamp(MIN_OPACITY, 1.0), settled);
            }
            &BrushIntent::StreamLine { amount, settled } if amount.is_finite() => {
                self.set(STREAMLINE, amount.clamp(0.0, 1.0), settled);
            }
            BrushIntent::Size { .. }
            | BrushIntent::Opacity { .. }
            | BrushIntent::StreamLine { .. } => {}
            BrushIntent::Edit { change, settled } => self.edit(change, *settled),
            BrushIntent::Rename(text) => {
                let key = self.brush(tool).key().clone();
                if self.library.rename(&key, text) {
                    self.changed();
                }
            }
            BrushIntent::Duplicate => self.duplicate(tool),
            BrushIntent::Reset => self.reset(tool),
            BrushIntent::Delete => self.delete(tool),
            BrushIntent::Reorder { key, to } => self.reorder(key, *to),
            BrushIntent::MoveToSet(set) => self.move_to_set(tool, *set),
        }
    }

    /// One of the tool's settings of its brush.
    fn set(&mut self, slot: usize, value: f32, settled: bool) {
        let tool = self.current();
        let brush = self.brush(tool);
        let (key, values) = (brush.key().clone(), self.setting(tool, brush));
        let setting = self.sets[tool.index()]
            .settings
            .entry(key)
            .or_insert(values);
        self.unsaved |= setting[slot] != value;
        setting[slot] = value;
        self.holding = !settled;
    }

    /// The library or the sets changed: the picker's rows follow, the store gets it.
    pub(super) fn changed(&mut self) {
        self.rows = rows_of(&self.library);
        self.set_rows = self.brush_sets.rows(&self.library);
        self.unsaved = true;
    }

    /// One setting of the tool's brush, held inside its range (`brush_edit.rs`).
    fn edit(&mut self, change: &BrushChange, settled: bool) {
        let brush = self.brush(self.current()).clone();
        let origin = self
            .library
            .origin(&brush)
            .map_or_else(|| brush.recipe().to_parts(), |o| o.recipe().to_parts());
        let mut kept = self
            .kept
            .take()
            .filter(|(key, _)| key == brush.key())
            .map(|(_, shape)| shape);
        if let Some(changed) = edited(&brush, change, (&mut kept, &origin)) {
            let size = changed.size();
            self.library.replace(changed);
            self.fit_sizes(brush.key(), size);
            self.changed();
        }
        self.kept = kept.map(|shape| (brush.key().clone(), shape));
        self.holding = !settled;
    }

    /// Every tool's size of the brush `key` moved inside `size`.
    fn fit_sizes(&mut self, key: &BrushKey, size: SizeRange) {
        for set in &mut self.sets {
            if let Some(setting) = set.settings.get_mut(key) {
                setting[SIZE] = size.clamp(setting[SIZE]);
            }
        }
    }

    /// A copy of the tool's brush right after it (in the library and in its set), chosen; every
    /// tool keeps for it what it set for the brush copied.
    fn duplicate(&mut self, tool: Tool) {
        let key = self.brush(tool).key().clone();
        let Some(copy) = self.library.duplicate(&key) else {
            return;
        };
        self.brush_sets.copied(&key, &copy);
        for set in &mut self.sets {
            if let Some(setting) = set.settings.get(&key).copied() {
                set.settings.insert(copy.clone(), setting);
            }
        }
        self.sets[tool.index()].current = copy;
        self.kept = None;
        self.changed();
    }

    /// The tool's brush, a built-in, as its file has it, and the tool's StreamLine on it.
    fn reset(&mut self, tool: Tool) {
        let key = self.brush(tool).key().clone();
        let Some(file) = self.library.file(&key).cloned() else {
            return;
        };
        let changed = self.library.reset(&key);
        self.fit_sizes(&key, file.size());
        let values = self.setting(tool, &file);
        let setting = self.sets[tool.index()]
            .settings
            .entry(key)
            .or_insert(values);
        let streamline = setting[STREAMLINE] != file.streamline();
        setting[STREAMLINE] = file.streamline();
        if changed || streamline {
            self.kept = None;
            self.changed();
        }
    }

    /// The tool's brush, a copy, goes; every tool that drew with it draws with its source.
    fn delete(&mut self, tool: Tool) {
        let key = self.brush(tool).key().clone();
        let Some(gone) = self.library.delete(&key) else {
            return;
        };
        self.brush_sets.deleted(&key);
        let instead = gone
            .source()
            .filter(|source| self.library.brush(source).is_some())
            .unwrap_or(self.library.default_key())
            .clone();
        for set in &mut self.sets {
            set.settings.remove(&key);
            if set.current == key {
                set.current = instead.clone();
            }
        }
        self.kept = None;
        self.changed();
    }
}
