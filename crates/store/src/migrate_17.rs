//! Schema 16 -> 17 (S41, D68): the toolbox record gains what the pen's two side buttons do and
//! the palette's swatches (`editor.rs`). A schema-16 toolbox gets the buttons' defaults, the
//! eraser and the eyedropper while held, and no swatches, so nothing draws differently after
//! the upgrade. No artwork record changes; heads are only marked 17, so a schema-16 build
//! reports a newer library rather than a damaged toolbox.

use openpaint_app::{ButtonMap, ProjectId};

use crate::codec::{Reader, Writer};
use crate::editor::{EDITOR, action_code, read_choice, read_colors};
use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 16 -> 17, from every project's head and the toolbox record as schema 16 stored them.
/// A head not marked 16, or a toolbox that is not a whole schema-16 one, is left as it is, so
/// opening it reports what is wrong (the toolbox then falls back to its defaults).
pub(crate) fn upgrade_16_to_17(heads: &[(ProjectId, Vec<u8>)], editor: Option<&[u8]>) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 16, 17) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    if let Some(editor) = editor.and_then(editor_16_to_17) {
        upgrade.library.push((EDITOR.to_vec(), editor));
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(17)));
    upgrade
}

/// A schema-9 to schema-16 toolbox as this build reads it (through schema 17), for the older
/// steps' tests.
#[cfg(test)]
pub(crate) fn upgraded_toolbox(editor: &[u8]) -> openpaint_app::EditorState {
    let upgraded = editor_16_to_17(editor).expect("a whole schema-16 toolbox");
    crate::migrate_18::upgraded_toolbox(&upgraded)
}

/// A whole schema-16 toolbox with the buttons' defaults and no swatches after it.
fn editor_16_to_17(editor: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(editor, "editor state");
    (reader.u8().ok()? <= 2).then_some(())?;
    for _ in 0..3 {
        read_choice(&mut reader).ok()?;
    }
    for _ in 0..3 {
        reader.f32().ok()?;
    }
    read_colors(&mut reader).ok()?;
    reader.end().ok()?;
    let defaults = ButtonMap::default();
    let mut writer = Writer::with_capacity(editor.len() + 6);
    writer.bytes(editor);
    writer.u8(action_code(defaults.primary));
    writer.u8(action_code(defaults.secondary));
    writer.u32(0);
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{write_choice, write_color};
    use crate::migrate_18::upgraded_toolbox;
    use openpaint_app::{BrushChoice, BrushSetting, ButtonAction, Tool};

    /// A schema-16 toolbox: the smudge current, a pencil setting, default eraser and smudge
    /// brushes, a colour and one recent colour.
    fn editor_16() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u8(2);
        let pencil = BrushChoice {
            brush: "pencil".into(),
            brushes: vec![BrushSetting {
                key: "pencil".into(),
                size: 4.0,
                opacity: 0.5,
                streamline: 0.25,
            }],
        };
        let empty = BrushChoice {
            brush: String::new(),
            brushes: Vec::new(),
        };
        for choice in [&pencil, &empty, &empty] {
            write_choice(&mut writer, choice);
        }
        write_color(&mut writer, [0.25, 0.5, 1.0]);
        writer.u32(1);
        write_color(&mut writer, [0.25, 0.5, 1.0]);
        writer.finish()
    }

    #[test]
    fn the_toolbox_gains_the_default_buttons_and_no_swatches_and_heads_are_marked() {
        let head = [&16u32.to_le_bytes()[..], &[1, 2, 3]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![9, 0, 0, 0, 7]),
        ];
        let old = editor_16();
        let upgrade = upgrade_16_to_17(&heads, Some(&old));
        let marked = [&17u32.to_le_bytes()[..], &[1, 2, 3]].concat();
        assert_eq!(
            upgrade.projects,
            [(head_key(ProjectId::new(1)), marked)],
            "a foreign head stays"
        );
        let (key, editor) = &upgrade.library[0];
        assert_eq!(key, EDITOR);
        assert_eq!(editor[..old.len()], old[..], "every old byte is kept");
        let editor = upgraded_toolbox(editor);
        assert_eq!(editor.tool, Tool::Smudge);
        assert_eq!(editor.paint.brushes[0].streamline, 0.25);
        assert_eq!((editor.color, editor.recent.len()), ([0.25, 0.5, 1.0], 1));
        assert_eq!(editor.buttons.primary, ButtonAction::Eraser);
        assert_eq!(editor.buttons.secondary, ButtonAction::Eyedropper);
        assert!(editor.swatches.is_empty());
        assert_eq!(upgrade.library[1], (SCHEMA_KEY.to_vec(), encode_schema(17)));
        for broken in [&old[..old.len() - 1], &[old.as_slice(), &[0]].concat()[..]] {
            let upgrade = upgrade_16_to_17(&[], Some(broken));
            assert_eq!(upgrade.library.len(), 1, "a broken toolbox stays as it is");
        }
        assert_eq!(upgrade_16_to_17(&[], None).library.len(), 1, "none: none");
    }
}
