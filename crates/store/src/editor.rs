//! The toolbox record, `library` / `editor` (schema 3, DECISIONS D27): the current brush's key
//! (u32 length + UTF-8), then the brush count (u32) and per brush its key, size and opacity
//! (f32). It is not artwork: the app falls back to the brushes' defaults when it cannot be
//! read, and nothing else depends on it.

use openpaint_app::{BrushSetting, EditorState, PortError};

use crate::codec::{Reader, Writer};

pub(crate) const EDITOR: &[u8] = b"editor";
/// Bytes of one brush entry with an empty key.
const SETTING_BYTES: usize = 4 + 4 + 4;

pub(crate) fn encode_editor(state: &EditorState) -> Vec<u8> {
    let mut writer = Writer::default();
    writer.str(&state.brush);
    writer.len(state.brushes.len());
    for setting in &state.brushes {
        writer.str(&setting.key);
        writer.f32(setting.size);
        writer.f32(setting.opacity);
    }
    writer.finish()
}

pub(crate) fn decode_editor(bytes: &[u8]) -> Result<EditorState, PortError> {
    let mut reader = Reader::new(bytes, "editor state");
    let brush = reader.str()?;
    let count = reader.count(SETTING_BYTES)?;
    let mut brushes = Vec::with_capacity(count);
    for _ in 0..count {
        brushes.push(BrushSetting {
            key: reader.str()?,
            size: reader.f32()?,
            opacity: reader.f32()?,
        });
    }
    reader.end()?;
    Ok(EditorState { brush, brushes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_round_trips_and_a_cut_record_is_corrupt() {
        let state = EditorState {
            brush: "pencil".into(),
            brushes: vec![
                BrushSetting {
                    key: "pencil".into(),
                    size: 6.5,
                    opacity: 0.25,
                },
                BrushSetting {
                    key: "ink-pen".into(),
                    size: 12.0,
                    opacity: 1.0,
                },
            ],
        };
        let bytes = encode_editor(&state);
        assert_eq!(decode_editor(&bytes), Ok(state));
        let corrupt = PortError::Corrupt {
            what: "editor state",
        };
        assert_eq!(decode_editor(&bytes[..bytes.len() - 1]), Err(corrupt));
    }
}
