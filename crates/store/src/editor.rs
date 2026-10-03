//! The toolbox record, `library` / `editor` (schema 6, DECISIONS D28, D29, D39): the tool (u8:
//! 0 brush, 1 eraser); the brush tool's choice, then the eraser's, each its brush's key (u32
//! length + UTF-8, empty for the default brush), the brush count (u32) and per brush its key,
//! size, opacity and StreamLine (f32); the colour, red, green, blue (f32); the recent colours, a
//! count (u32) and per colour red, green, blue (f32), newest first. It is not artwork: the app
//! falls back to the defaults when it cannot be read, and nothing else depends on it. Schema 3
//! held only the brush tool's choice, schemas 4 and 5 no StreamLine; `migrate.rs` and
//! `migrate_6.rs` upgrade them.

use openpaint_app::{BrushChoice, BrushSetting, EditorState, PortError, Tool};

use crate::codec::{Reader, Writer};

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
    });
    for choice in [&state.paint, &state.erase] {
        write_choice(&mut writer, choice);
    }
    write_color(&mut writer, state.color);
    writer.len(state.recent.len());
    for color in &state.recent {
        write_color(&mut writer, *color);
    }
    writer.finish()
}

/// One tool's brush choice as the record holds it; schema 3's whole record was one of these,
/// without the StreamLine.
pub(crate) fn write_choice(writer: &mut Writer, choice: &BrushChoice) {
    writer.str(&choice.brush);
    writer.len(choice.brushes.len());
    for setting in &choice.brushes {
        writer.str(&setting.key);
        writer.f32(setting.size);
        writer.f32(setting.opacity);
        writer.f32(setting.streamline);
    }
}

pub(crate) fn write_color(writer: &mut Writer, [r, g, b]: [f32; 3]) {
    for channel in [r, g, b] {
        writer.f32(channel);
    }
}

pub(crate) fn decode_editor(bytes: &[u8]) -> Result<EditorState, PortError> {
    let mut reader = Reader::new(bytes, "editor state");
    let tool = match reader.u8()? {
        0 => Tool::Brush,
        1 => Tool::Eraser,
        _ => return Err(reader.corrupt()),
    };
    let paint = read_choice(&mut reader)?;
    let erase = read_choice(&mut reader)?;
    let color = read_color(&mut reader)?;
    let count = reader.count(COLOR_BYTES)?;
    let recent = (0..count)
        .map(|_| read_color(&mut reader))
        .collect::<Result<_, _>>()?;
    reader.end()?;
    Ok(EditorState {
        tool,
        paint,
        erase,
        color,
        recent,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_round_trips_and_a_cut_record_is_corrupt() {
        let setting = |key: &str, size, opacity| BrushSetting {
            key: key.into(),
            size,
            opacity,
            streamline: 0.75,
        };
        let state = EditorState {
            tool: Tool::Eraser,
            paint: BrushChoice {
                brush: "pencil".into(),
                brushes: vec![setting("pencil", 6.5, 0.25), setting("ink-pen", 12.0, 1.0)],
            },
            erase: BrushChoice {
                brush: String::new(),
                brushes: Vec::new(),
            },
            color: [0.25, 0.5, 1.0],
            recent: vec![[0.25, 0.5, 1.0], [0.0, 0.0, 0.0]],
        };
        let bytes = encode_editor(&state);
        assert_eq!(decode_editor(&bytes), Ok(state));
        let corrupt = PortError::Corrupt {
            what: "editor state",
        };
        assert_eq!(
            decode_editor(&bytes[..bytes.len() - 1]),
            Err(corrupt.clone())
        );
        let mut unknown_tool = bytes;
        unknown_tool[0] = 2;
        assert_eq!(decode_editor(&unknown_tool), Err(corrupt));
    }
}
