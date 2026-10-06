//! The toolboxes of the frozen samples, as each schema holds them once opened now.

use openpaint_app::{BrushChoice, BrushSetting, ButtonAction, ButtonMap, EditorState, Tool};

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

/// The toolbox of the schema-17 sample: the schema-9 one with the side buttons undoing and
/// switching tools and three swatches.
pub fn sample_editor_17() -> EditorState {
    EditorState {
        buttons: ButtonMap {
            primary: ButtonAction::Undo,
            secondary: ButtonAction::SwitchTool,
        },
        swatches: vec![[1.0, 0.5, 0.125], [0.0, 0.25, 0.75], [1.0, 1.0, 1.0]],
        ..sample_editor_9()
    }
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
    }
}
