//! The toolbox as plain data for the store (`Toolbox::state`, `Toolbox::restore`).

use crate::toolbox::Tool;

#[derive(Debug, Clone, PartialEq)]
pub struct EditorState {
    pub tool: Tool,
    pub paint: BrushChoice,
    pub erase: BrushChoice,
    pub smudge: BrushChoice,
    /// sRGB red, green, blue.
    pub color: [f32; 3],
    /// Newest first.
    pub recent: Vec<[f32; 3]>,
}

/// One tool's brush and each brush's size, opacity and StreamLine for it, by key. An empty
/// `brush` means the tool's default brush.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushChoice {
    pub brush: String,
    pub brushes: Vec<BrushSetting>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrushSetting {
    pub key: String,
    pub size: f32,
    /// The opacity; the smudge's strength.
    pub opacity: f32,
    /// The StreamLine amount, `0..=1`.
    pub streamline: f32,
}
