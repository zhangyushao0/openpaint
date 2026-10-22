//! The sample toolbox: every field of the toolbox record set.

use openpaint_app::{
    BrushChoice, BrushParts, BrushSetting, ButtonAction, ButtonMap, EditorState, StoredBrush, Tool,
};
use openpaint_core::{BrushRecipe, ShapeParts, ShapeTexture, Stamp};

fn setting(key: &str, size: f32, opacity: f32, streamline: f32) -> BrushSetting {
    BrushSetting {
        key: key.to_owned(),
        size,
        opacity,
        streamline,
    }
}

/// The brush library: the Pencil by key, the Inker edited (a longer taper), a copy of it laying
/// chalk stamps 0.4 diameters apart with its own name, the Ink Pen by key after it.
fn library() -> Vec<StoredBrush> {
    let by_key = |key: &str| StoredBrush {
        key: key.to_owned(),
        brush: None,
    };
    let mut inker = BrushRecipe::plain().to_parts();
    inker.taper = [5.0, 3.0, 0.1];
    let mut chalk = inker.clone();
    chalk.shape = ShapeParts {
        stamp: Stamp::Texture(ShapeTexture::Chalk),
        spacing: 0.4,
        ..ShapeParts::CONTINUOUS
    };
    let whole = |name: &str, source: &str, recipe| BrushParts {
        name: name.to_owned(),
        source: source.to_owned(),
        size: [1.0, 12.0, 80.0],
        opacity: 1.0,
        streamline: 0.25,
        recipe,
    };
    vec![
        by_key("pencil"),
        StoredBrush {
            key: "inker".to_owned(),
            brush: Some(whole("Inker", "", inker)),
        },
        StoredBrush {
            key: "copy-1".to_owned(),
            brush: Some(whole("Kreide ✒ 粉笔", "inker", chalk)),
        },
        by_key("ink-pen"),
    ]
}

/// The smudge current on the soft brush at 70 % with some StreamLine; the brush tool drawing
/// with the chalk copy, the pencil small and light, StreamLine on the ink pen; the eraser on the
/// soft brush at half opacity; the library above, its brushes arranged in sets (the copy moved
/// to Sketching, the Inker before the Ink Pen, Speckle first in Painting) and three brushes drawn
/// with, the copy last; an orange colour, two recent ones, three swatches; the side buttons
/// undoing, switching tools and picking.
pub fn sample_editor() -> EditorState {
    let keys = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
    EditorState {
        tool: Tool::Smudge,
        paint: BrushChoice {
            brush: "copy-1".into(),
            brushes: vec![
                setting("pencil", 3.5, 0.25, 0.0),
                setting("ink-pen", 12.0, 1.0, 0.625),
                setting("copy-1", 20.0, 0.75, 0.0),
            ],
        },
        erase: BrushChoice {
            brush: "soft-brush".into(),
            brushes: vec![setting("soft-brush", 64.0, 0.5, 1.0)],
        },
        smudge: BrushChoice {
            brush: "soft-brush".into(),
            brushes: vec![setting("soft-brush", 90.0, 0.7, 0.5)],
        },
        color: [1.0, 0.5, 0.125],
        recent: vec![[1.0, 0.5, 0.125], [0.0, 0.25, 0.75]],
        buttons: ButtonMap {
            primary: ButtonAction::Undo,
            secondary: ButtonAction::SwitchTool,
            tertiary: ButtonAction::Eyedropper,
        },
        swatches: vec![[1.0, 0.5, 0.125], [0.0, 0.25, 0.75], [1.0, 1.0, 1.0]],
        library: library(),
        sets: [
            keys(&["pencil", "copy-1"]),
            keys(&["inker", "ink-pen", "monoline"]),
            keys(&[
                "speckle",
                "hard-round",
                "soft-brush",
                "flat",
                "opaque-paint",
            ]),
            keys(&["textured-round"]),
        ],
        recent_brushes: keys(&["copy-1", "speckle", "ink-pen"]),
    }
}
