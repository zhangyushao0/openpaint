//! The toolbox record, `library` / `editor` (format 1, DECISIONS D70, D74): the tool (u8: 0
//! brush, 1 eraser, 2 smudge); the brush tool's choice, then the eraser's, then the smudge's, each
//! its brush's key (u32 length + UTF-8, empty for the tool's default brush), the brush count (u32)
//! and per brush its key, size, opacity and StreamLine (f32); the brush library, the brush sets
//! and Recent (`editor_library.rs`); the colour, red, green, blue (f32); the recent colours, a
//! count (u32) and per colour red, green, blue (f32), newest first; the palette's swatches alike,
//! in their order; what the pen's first, second and third side button do (u8 each: 0 nothing, 1
//! eraser while held, 2 eyedropper while held, 3 undo, 4 redo, 5 switch tool); then, once a
//! language was chosen (added in 1.2, D100), the interface's language (u8: 0 English, 1
//! Chinese): a record without it, 1.1's or one written before the choice, follows the system's.
//! It is not artwork: the app falls back to the defaults when it cannot be read, and nothing
//! else depends on it.

use openpaint_app::{
    BrushChoice, BrushSetting, ButtonAction, ButtonMap, EditorState, Language, PortError, Tool,
};

use crate::codec::{Reader, Writer};
use crate::editor_library::{read_library, read_sets, write_library, write_sets};

pub(crate) const EDITOR: &[u8] = b"editor";
/// Bytes of one brush entry with an empty key.
const SETTING_BYTES: usize = 4 + 4 + 4 + 4;
/// Bytes of one colour.
const COLOR_BYTES: usize = 3 * 4;

pub(crate) fn encode_editor(state: &EditorState) -> Vec<u8> {
    let mut writer = Writer::default();
    writer.u8(match state.tool {
        Tool::Brush => 0,
        Tool::Eraser => 1,
        Tool::Smudge => 2,
    });
    for choice in [&state.paint, &state.erase, &state.smudge] {
        write_choice(&mut writer, choice);
    }
    write_library(&mut writer, &state.library);
    write_sets(&mut writer, (&state.sets, &state.recent_brushes));
    write_color(&mut writer, state.color);
    write_colors(&mut writer, &state.recent);
    write_colors(&mut writer, &state.swatches);
    let buttons = state.buttons;
    for action in [buttons.primary, buttons.secondary, buttons.tertiary] {
        writer.u8(action_code(action));
    }
    if let Some(language) = state.language {
        writer.u8(language_code(language));
    }
    writer.finish()
}

/// A language as the record holds it.
const fn language_code(language: Language) -> u8 {
    match language {
        Language::English => 0,
        Language::Chinese => 1,
    }
}

/// The language ending the record, if one was chosen.
fn read_language(reader: &mut Reader<'_>) -> Result<Option<Language>, PortError> {
    if reader.is_done() {
        return Ok(None);
    }
    let code = reader.u8()?;
    Language::ALL
        .into_iter()
        .find(|language| language_code(*language) == code)
        .map(Some)
        .ok_or_else(|| reader.corrupt())
}

/// One tool's brush choice as the record holds it.
fn write_choice(writer: &mut Writer, choice: &BrushChoice) {
    writer.str(&choice.brush);
    writer.len(choice.brushes.len());
    for setting in &choice.brushes {
        writer.str(&setting.key);
        writer.f32(setting.size);
        writer.f32(setting.opacity);
        writer.f32(setting.streamline);
    }
}

fn write_color(writer: &mut Writer, [r, g, b]: [f32; 3]) {
    for channel in [r, g, b] {
        writer.f32(channel);
    }
}

fn write_colors(writer: &mut Writer, colors: &[[f32; 3]]) {
    writer.len(colors.len());
    for color in colors {
        write_color(writer, *color);
    }
}

/// A button's action as the record holds it.
const fn action_code(action: ButtonAction) -> u8 {
    match action {
        ButtonAction::Nothing => 0,
        ButtonAction::Eraser => 1,
        ButtonAction::Eyedropper => 2,
        ButtonAction::Undo => 3,
        ButtonAction::Redo => 4,
        ButtonAction::SwitchTool => 5,
    }
}

fn read_action(reader: &mut Reader<'_>) -> Result<ButtonAction, PortError> {
    let code = reader.u8()?;
    ButtonAction::ALL
        .into_iter()
        .find(|action| action_code(*action) == code)
        .ok_or_else(|| reader.corrupt())
}

pub(crate) fn decode_editor(bytes: &[u8]) -> Result<EditorState, PortError> {
    let mut reader = Reader::new(bytes, "editor state");
    let tool = match reader.u8()? {
        0 => Tool::Brush,
        1 => Tool::Eraser,
        2 => Tool::Smudge,
        _ => return Err(reader.corrupt()),
    };
    let paint = read_choice(&mut reader)?;
    let erase = read_choice(&mut reader)?;
    let smudge = read_choice(&mut reader)?;
    let library = read_library(&mut reader)?;
    let (sets, recent_brushes) = read_sets(&mut reader)?;
    let color = read_color(&mut reader)?;
    let recent = read_colors(&mut reader)?;
    let swatches = read_colors(&mut reader)?;
    let buttons = ButtonMap {
        primary: read_action(&mut reader)?,
        secondary: read_action(&mut reader)?,
        tertiary: read_action(&mut reader)?,
    };
    let language = read_language(&mut reader)?;
    reader.end()?;
    Ok(EditorState {
        tool,
        paint,
        erase,
        smudge,
        color,
        recent,
        buttons,
        swatches,
        library,
        sets,
        recent_brushes,
        language,
    })
}

fn read_choice(reader: &mut Reader<'_>) -> Result<BrushChoice, PortError> {
    let brush = reader.str()?;
    let count = reader.count(SETTING_BYTES)?;
    let mut brushes = Vec::with_capacity(count);
    for _ in 0..count {
        brushes.push(BrushSetting {
            key: reader.str()?,
            size: reader.f32()?,
            opacity: reader.f32()?,
            streamline: reader.f32()?,
        });
    }
    Ok(BrushChoice { brush, brushes })
}

fn read_color(reader: &mut Reader<'_>) -> Result<[f32; 3], PortError> {
    Ok([reader.f32()?, reader.f32()?, reader.f32()?])
}

fn read_colors(reader: &mut Reader<'_>) -> Result<Vec<[f32; 3]>, PortError> {
    let count = reader.count(COLOR_BYTES)?;
    (0..count).map(|_| read_color(reader)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smudge current on its soft brush, the pencil the brush's, two recent colours, every
    /// button doing something, two swatches; the built-ins where a first start puts them.
    fn state() -> EditorState {
        let setting = |key: &str, size, opacity| BrushSetting {
            key: key.into(),
            size,
            opacity,
            streamline: 0.75,
        };
        EditorState {
            tool: Tool::Smudge,
            paint: BrushChoice {
                brush: "pencil".into(),
                brushes: vec![setting("pencil", 6.5, 0.25), setting("ink-pen", 12.0, 1.0)],
            },
            erase: BrushChoice {
                brush: String::new(),
                brushes: Vec::new(),
            },
            smudge: BrushChoice {
                brush: "soft-brush".into(),
                brushes: vec![setting("soft-brush", 90.0, 0.7)],
            },
            color: [0.25, 0.5, 1.0],
            recent: vec![[0.25, 0.5, 1.0], [0.0, 0.0, 0.0]],
            buttons: ButtonMap {
                primary: ButtonAction::SwitchTool,
                secondary: ButtonAction::Redo,
                tertiary: ButtonAction::Eyedropper,
            },
            swatches: vec![[1.0, 0.0, 0.0], [0.0, 0.5, 0.25]],
            library: Vec::new(),
            sets: Default::default(),
            recent_brushes: Vec::new(),
            language: None,
        }
    }

    #[test]
    fn a_chosen_language_ends_the_record_and_none_writes_nothing() {
        let none = encode_editor(&state());
        for (language, code) in [(Language::English, 0), (Language::Chinese, 1)] {
            let chosen = EditorState {
                language: Some(language),
                ..state()
            };
            let bytes = encode_editor(&chosen);
            assert_eq!((bytes.len(), bytes[none.len()]), (none.len() + 1, code));
            assert_eq!(decode_editor(&bytes), Ok(chosen));
        }
        let mut unknown = none.clone();
        unknown.push(2);
        let corrupt = PortError::Corrupt {
            what: "editor state",
        };
        assert_eq!(decode_editor(&unknown), Err(corrupt.clone()));
        unknown.truncate(none.len());
        unknown.extend([1, 0]);
        assert_eq!(decode_editor(&unknown), Err(corrupt));
    }

    #[test]
    fn the_state_round_trips_and_a_cut_record_is_corrupt() {
        let state = state();
        let bytes = encode_editor(&state);
        assert_eq!(decode_editor(&bytes), Ok(state.clone()));
        let corrupt = PortError::Corrupt {
            what: "editor state",
        };
        assert_eq!(
            decode_editor(&bytes[..bytes.len() - 1]),
            Err(corrupt.clone())
        );
        let mut unknown_tool = bytes.clone();
        unknown_tool[0] = 3;
        assert_eq!(decode_editor(&unknown_tool), Err(corrupt.clone()));
        // The buttons end the record.
        let at = bytes.len() - 3;
        assert_eq!(bytes[at..], [5, 4, 2]);
        let mut unknown_action = bytes;
        unknown_action[at + 2] = 6;
        assert_eq!(decode_editor(&unknown_action), Err(corrupt));
        for action in ButtonAction::ALL {
            let state = EditorState {
                buttons: ButtonMap {
                    primary: action,
                    secondary: action,
                    tertiary: action,
                },
                ..state.clone()
            };
            assert_eq!(decode_editor(&encode_editor(&state)), Ok(state));
        }
    }
}
