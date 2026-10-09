//! Schema 17 -> 18 (S43, D70): the toolbox record gains what the pen's third side button does,
//! right after the second's (`editor.rs`). A schema-17 toolbox gets the third button's default,
//! Undo, every old byte kept around it, so the first two buttons and the swatches stay as they
//! were. No artwork record changes; heads are only marked 18, so a schema-17 build reports a
//! newer library rather than a damaged toolbox.

use openpaint_app::{ButtonMap, ProjectId};

use crate::codec::{Reader, Writer};
use crate::editor::{EDITOR, action_code, read_action, read_choice, read_colors};
use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 17 -> 18, from every project's head and the toolbox record as schema 17 stored them.
/// A head not marked 17, or a toolbox that is not a whole schema-17 one, is left as it is, so
/// opening it reports what is wrong (the toolbox then falls back to its defaults).
pub(crate) fn upgrade_17_to_18(heads: &[(ProjectId, Vec<u8>)], editor: Option<&[u8]>) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 17, 18) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    if let Some(editor) = editor.and_then(editor_17_to_18) {
        upgrade.library.push((EDITOR.to_vec(), editor));
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(18)));
    upgrade
}

/// A schema-17 toolbox as this build reads it, for the older steps' tests.
#[cfg(test)]
pub(crate) fn upgraded_toolbox(editor: &[u8]) -> openpaint_app::EditorState {
    let upgraded = editor_17_to_18(editor).expect("a whole schema-17 toolbox");
    crate::editor::decode_editor(&upgraded).expect("a schema-18 toolbox")
}

/// A whole schema-17 toolbox with the third button's default after its second button.
fn editor_17_to_18(editor: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(editor, "editor state");
    (reader.u8().ok()? <= 2).then_some(())?;
    for _ in 0..3 {
        read_choice(&mut reader).ok()?;
    }
    for _ in 0..3 {
        reader.f32().ok()?;
    }
    read_colors(&mut reader).ok()?;
    for _ in 0..2 {
        read_action(&mut reader).ok()?;
    }
    let swatches = reader.rest();
    let mut rest = Reader::new(swatches, "editor state");
    read_colors(&mut rest).ok()?;
    rest.end().ok()?;
    let at = editor.len() - swatches.len();
    let mut writer = Writer::with_capacity(editor.len() + 1);
    writer.bytes(&editor[..at]);
    writer.u8(action_code(ButtonMap::default().tertiary));
    writer.bytes(swatches);
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{decode_editor, write_choice, write_color};
    use openpaint_app::{BrushChoice, ButtonAction, Tool};

    /// A schema-17 toolbox: the eraser current, default brushes, a colour, no recent colours,
    /// the side buttons on Redo and Switch tool, two swatches.
    fn editor_17() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u8(1);
        let empty = BrushChoice {
            brush: String::new(),
            brushes: Vec::new(),
        };
        for _ in 0..3 {
            write_choice(&mut writer, &empty);
        }
        write_color(&mut writer, [0.5, 0.25, 0.0]);
        writer.u32(0);
        writer.u8(action_code(ButtonAction::Redo));
        writer.u8(action_code(ButtonAction::SwitchTool));
        writer.u32(2);
        write_color(&mut writer, [1.0, 0.0, 0.0]);
        write_color(&mut writer, [0.0, 0.0, 1.0]);
        writer.finish()
    }

    #[test]
    fn the_toolbox_gains_the_third_button_undoing_and_heads_are_marked() {
        let head = [&17u32.to_le_bytes()[..], &[4, 5, 6]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![16, 0, 0, 0, 7]),
        ];
        let old = editor_17();
        let upgrade = upgrade_17_to_18(&heads, Some(&old));
        let marked = [&18u32.to_le_bytes()[..], &[4, 5, 6]].concat();
        assert_eq!(
            upgrade.projects,
            [(head_key(ProjectId::new(1)), marked)],
            "a foreign head stays"
        );
        let (key, editor) = &upgrade.library[0];
        assert_eq!(key, EDITOR);
        // The swatches: their count and two colours.
        let at = old.len() - 4 - 2 * 12;
        assert_eq!(editor[..at], old[..at], "every byte before the swatches");
        assert_eq!(editor[at], action_code(ButtonAction::Undo), "the third");
        assert_eq!(editor[at + 1..], old[at..], "the swatches as they were");
        let editor = decode_editor(editor).expect("a schema-18 toolbox");
        assert_eq!(editor.tool, Tool::Eraser);
        assert_eq!(editor.color, [0.5, 0.25, 0.0]);
        let buttons = ButtonMap {
            primary: ButtonAction::Redo,
            secondary: ButtonAction::SwitchTool,
            tertiary: ButtonAction::Undo,
        };
        assert_eq!(editor.buttons, buttons);
        assert_eq!(editor.swatches, [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
        assert_eq!(upgrade.library[1], (SCHEMA_KEY.to_vec(), encode_schema(18)));
        let mut unknown = old.clone();
        unknown[at - 1] = 6;
        for broken in [
            &old[..old.len() - 1],
            &[old.as_slice(), &[0]].concat()[..],
            &unknown[..],
        ] {
            let upgrade = upgrade_17_to_18(&[], Some(broken));
            assert_eq!(upgrade.library.len(), 1, "a broken toolbox stays as it is");
        }
        assert_eq!(upgrade_17_to_18(&[], None).library.len(), 1, "none: none");
    }
}
