//! The toolbox as plain data for the store (`Toolbox::state`, `Toolbox::restore`).

use openpaint_core::RecipeParts;

use crate::pen_buttons::ButtonMap;
use crate::toolbox::Tool;

#[derive(Debug, Clone, PartialEq)]
pub struct EditorState {
    /// The tool chosen; never one a pen button only holds.
    pub tool: Tool,
    pub paint: BrushChoice,
    pub erase: BrushChoice,
    pub smudge: BrushChoice,
    /// sRGB red, green, blue.
    pub color: [f32; 3],
    /// Newest first.
    pub recent: Vec<[f32; 3]>,
    /// What the pen's side buttons do (S41).
    pub buttons: ButtonMap,
    /// The palette's swatches, sRGB, in their order (S41).
    pub swatches: Vec<[f32; 3]>,
    /// The brush library in picker order (S46); empty: the built-ins as their files have them.
    pub library: Vec<StoredBrush>,
    /// The keys of Sketching, Inking, Painting and Texture in their order (S47); all four empty
    /// while every brush lies where a first start puts it. A brush in no list goes there.
    pub sets: [Vec<String>; 4],
    /// The keys of the brushes drawn with last, newest first (S47).
    pub recent_brushes: Vec<String>,
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

/// One brush of the library: its key, and its values unless it is a built-in as its file has
/// it.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBrush {
    pub key: String,
    pub brush: Option<BrushParts>,
}

/// A brush as plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushParts {
    pub name: String,
    /// The key of the brush a copy was made from; empty for a built-in.
    pub source: String,
    /// The size slider's least, first and largest diameter, in canvas pixels.
    pub size: [f32; 3],
    /// The opacity the slider starts at.
    pub opacity: f32,
    /// The StreamLine every tool starts with.
    pub streamline: f32,
    pub recipe: RecipeParts,
}
