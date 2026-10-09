//! Taking over the stored toolbox: the brush library first (S46), then each tool's brush and its
//! settings per brush, the tool, the pen's buttons and the colours.

use crate::brushes::BrushKey;
use crate::editor_state::{BrushChoice, EditorState};
use crate::streamline::StreamLine;

use super::{MIN_OPACITY, Tool, Toolbox, rows_of};

impl Toolbox {
    /// Takes over a stored state: its library first (a stored brush the engine refuses falls
    /// back to its built-in, `fell_back`), then each tool's brush and settings. Unknown
    /// brushes, values outside a brush's ranges and colours out of range are skipped (those keep
    /// their defaults). Returns whether everything was taken over.
    pub fn restore(&mut self, state: &EditorState) -> bool {
        let mut fell_back = Vec::new();
        let library = self.library.restore(&state.library, &mut fell_back);
        self.fell_back = fell_back;
        self.rows = rows_of(&self.library);
        self.kept = None;
        let brushes = &self.library;
        for set in &mut self.sets {
            if brushes.brush(&set.current).is_none() {
                set.current = brushes.default_key().clone();
            }
        }
        self.tool = state.tool;
        self.buttons = state.buttons;
        let paint = self.restore_set(Tool::Brush, &state.paint);
        let erase = self.restore_set(Tool::Eraser, &state.erase);
        let smudge = self.restore_set(Tool::Smudge, &state.smudge);
        let colors = self
            .palette
            .restore(state.color, &state.recent, &state.swatches);
        library && paint && erase && smudge && colors
    }

    /// The names of the stored brushes the engine refused at the last restore, which fell back
    /// to the built-ins they come from.
    #[must_use]
    pub fn fell_back(&self) -> &[String] {
        &self.fell_back
    }

    fn restore_set(&mut self, tool: Tool, choice: &BrushChoice) -> bool {
        let mut whole = true;
        let library = &self.library;
        let set = &mut self.sets[tool.index()];
        set.settings.clear();
        for setting in &choice.brushes {
            let Some(brush) = library.brush(&BrushKey::new(&setting.key)) else {
                whole = false;
                continue;
            };
            let opacity = (MIN_OPACITY..=1.0).contains(&setting.opacity);
            let streamline = StreamLine::new(setting.streamline).is_some();
            if brush.size().contains(setting.size) && opacity && streamline {
                let values = [setting.size, setting.opacity, setting.streamline];
                set.settings.insert(brush.key().clone(), values);
            } else {
                whole = false;
            }
        }
        if !choice.brush.is_empty() {
            match library.brush(&BrushKey::new(&choice.brush)) {
                Some(brush) => set.current = brush.key().clone(),
                None => whole = false,
            }
        }
        whole
    }
}
