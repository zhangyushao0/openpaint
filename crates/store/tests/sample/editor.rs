//! The toolboxes of the frozen samples, as each schema holds them once opened now.

use openpaint_app::{
    BrushChoice, BrushParts, BrushSetting, ButtonAction, ButtonMap, EditorState, StoredBrush, Tool,
};
use openpaint_core::{BrushRecipe, ShapeParts, ShapeTexture, Stamp};

fn setting(key: &str, size: f32, opacity: f32) -> BrushSetting {
    BrushSetting {
        key: key.to_owned(),
        size,
        opacity,
        streamline: 0.0,
    }
}

/// The brush tool's choice of the schema-3 sample: the pencil current, small and light, the
/// ink pen at its default; StreamLine off, as every brush had it before schema 6.
fn pencil_choice() -> BrushChoice {
    BrushChoice {
        brush: "pencil".into(),
        brushes: vec![setting("pencil", 3.5, 0.25), setting("ink-pen", 12.0, 1.0)],
    }
}

/// No brush chosen and no brush set: the tool's defaults.
fn default_choice() -> BrushChoice {
    BrushChoice {
        brush: String::new(),
        brushes: Vec::new(),
    }
}

/// The schema-3 sample's toolbox as schema 4 reads it after the upgrade.
pub fn upgraded_editor() -> EditorState {
    EditorState {
        tool: Tool::Brush,
        paint: pencil_choice(),
        erase: default_choice(),
        smudge: default_choice(),
        color: [0.0; 3],
        recent: Vec::new(),
        buttons: ButtonMap::default(),
        swatches: Vec::new(),
        library: Vec::new(),
        sets: Default::default(),
        recent_brushes: Vec::new(),
    }
}

/// The toolbox of the schema-9 sample: the schema-6 one with the smudge current, on the soft
/// brush at 70 % with some StreamLine.
pub fn sample_editor_9() -> EditorState {
    let mut editor = sample_editor_6();
    editor.tool = Tool::Smudge;
    editor.smudge = BrushChoice {
        brush: "soft-brush".into(),
        brushes: vec![BrushSetting {
            streamline: 0.5,
            ..setting("soft-brush", 90.0, 0.7)
        }],
    };
    editor
}

/// The toolbox of the schema-17 sample as this build opens it: the schema-9 one with the side
/// buttons undoing and switching tools, the third at its default (Undo), and three swatches.
pub fn sample_editor_17() -> EditorState {
    EditorState {
        buttons: ButtonMap {
            primary: ButtonAction::Undo,
            secondary: ButtonAction::SwitchTool,
            tertiary: ButtonAction::Undo,
        },
        swatches: vec![[1.0, 0.5, 0.125], [0.0, 0.25, 0.75], [1.0, 1.0, 1.0]],
        ..sample_editor_9()
    }
}

/// The toolbox of the schema-18 to 20 samples (schemas 19 and 20 changed no toolbox): the
/// schema-17 one with the third side button holding the eyedropper.
pub fn sample_editor_18() -> EditorState {
    let mut editor = sample_editor_17();
    editor.buttons.tertiary = ButtonAction::Eyedropper;
    editor
}

/// The toolbox of the schema-6 sample: the schema-5 one with StreamLine on the ink pen and on
/// the eraser's soft brush.
pub fn sample_editor_6() -> EditorState {
    let mut editor = sample_editor();
    editor.paint.brushes[1].streamline = 0.625;
    editor.erase.brushes[0].streamline = 1.0;
    editor
}

/// The eraser current on the soft brush at half opacity; an orange colour, two recent ones.
/// The toolbox of the schema-4 and schema-5 samples.
pub fn sample_editor() -> EditorState {
    EditorState {
        tool: Tool::Eraser,
        paint: pencil_choice(),
        erase: BrushChoice {
            brush: "soft-brush".into(),
            brushes: vec![setting("soft-brush", 64.0, 0.5)],
        },
        smudge: default_choice(),
        color: [1.0, 0.5, 0.125],
        recent: vec![[1.0, 0.5, 0.125], [0.0, 0.25, 0.75]],
        buttons: ButtonMap::default(),
        swatches: Vec::new(),
        library: Vec::new(),
        sets: Default::default(),
        recent_brushes: Vec::new(),
    }
}

/// The toolbox of the schema-21 sample: the schema-18 one with a brush library: the Pencil by
/// key, the Inker edited (a longer taper), a copy of it laying chalk stamps 0.4 diameters apart
/// with its own name, the Ink Pen by key after it, and the brush tool drawing with the copy.
pub fn sample_editor_21() -> EditorState {
    let mut editor = sample_editor_18();
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
    editor.library = vec![
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
    ];
    editor.paint.brush = "copy-1".to_owned();
    editor.paint.brushes.push(setting("copy-1", 20.0, 0.75));
    editor
}

/// The toolbox of the schema-22 sample: the schema-21 one with its brushes arranged in sets (the
/// copy moved to Sketching, the Inker before the Ink Pen, Speckle first in Painting) and three
/// brushes drawn with, the copy last.
pub fn sample_editor_22() -> EditorState {
    let keys = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
    EditorState {
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
        ..sample_editor_21()
    }
}
