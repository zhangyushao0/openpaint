//! Schema 21 -> 22 (S47, DECISIONS D74): the toolbox record gains the brush sets and Recent after
//! the brush library (`editor_library.rs`). A schema-21 toolbox gets four empty sets, which put
//! every brush where a first start does (the built-ins by their jobs, a copy right after its
//! source), and an empty Recent; every old byte stays. No artwork record changes; heads are only
//! marked 22, so a schema-21 build reports a newer library rather than a damaged toolbox.

use openpaint_app::ProjectId;

use crate::codec::{Reader, Writer};
use crate::editor::{EDITOR, read_action, read_choice, read_colors};
use crate::editor_library::read_library;
use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Counts the step appends: four sets and Recent, each empty.
const EMPTY_SETS: usize = 5;

/// Schema 21 -> 22, from every project's head and the toolbox record as schema 21 stored them.
/// A head not marked 21, or a toolbox that is not a whole schema-21 one, is left as it is, so
/// opening it reports what is wrong (the toolbox then falls back to its defaults).
pub(crate) fn upgrade_21_to_22(heads: &[(ProjectId, Vec<u8>)], editor: Option<&[u8]>) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 21, 22) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    if let Some(editor) = editor.and_then(editor_21_to_22) {
        upgrade.library.push((EDITOR.to_vec(), editor));
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(22)));
    upgrade
}

/// A schema-18 to 21 toolbox as this build reads it, for the older steps' tests.
#[cfg(test)]
pub(crate) fn upgraded_toolbox(editor: &[u8]) -> openpaint_app::EditorState {
    let upgraded = editor_21_to_22(editor).expect("a whole schema-21 toolbox");
    crate::editor::decode_editor(&upgraded).expect("a schema-22 toolbox")
}

/// A whole schema-21 toolbox with four empty sets and an empty Recent after its library.
fn editor_21_to_22(editor: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(editor, "editor state");
    (reader.u8().ok()? <= 2).then_some(())?;
    for _ in 0..3 {
        read_choice(&mut reader).ok()?;
    }
    for _ in 0..3 {
        reader.f32().ok()?;
    }
    read_colors(&mut reader).ok()?;
    for _ in 0..3 {
        read_action(&mut reader).ok()?;
    }
    read_colors(&mut reader).ok()?;
    read_library(&mut reader).ok()?;
    reader.end().ok()?;
    let mut writer = Writer::with_capacity(editor.len() + 4 * EMPTY_SETS);
    writer.bytes(editor);
    for _ in 0..EMPTY_SETS {
        writer.u32(0);
    }
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{action_code, write_choice, write_color};
    use crate::editor_library::write_library;
    use openpaint_app::{BrushChoice, ButtonAction, StoredBrush, Tool};

    /// A schema-21 toolbox: the eraser current, default brushes, a colour, no recent colours,
    /// the side buttons on Undo, Redo and the eyedropper, no swatch, a library of the Pencil and
    /// the Ink Pen by key.
    fn editor_21() -> Vec<u8> {
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
        for action in [
            ButtonAction::Undo,
            ButtonAction::Redo,
            ButtonAction::Eyedropper,
        ] {
            writer.u8(action_code(action));
        }
        writer.u32(0);
        let by_key = |key: &str| StoredBrush {
            key: key.to_owned(),
            brush: None,
        };
        write_library(&mut writer, &[by_key("pencil"), by_key("ink-pen")]);
        writer.finish()
    }

    #[test]
    fn the_toolbox_gains_empty_sets_and_recent_and_heads_are_marked() {
        let head = [&21u32.to_le_bytes()[..], &[4, 5, 6]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![20, 0, 0, 0, 7]),
        ];
        let old = editor_21();
        let upgrade = upgrade_21_to_22(&heads, Some(&old));
        let marked = [&22u32.to_le_bytes()[..], &[4, 5, 6]].concat();
        assert_eq!(
            upgrade.projects,
            [(head_key(ProjectId::new(1)), marked)],
            "a foreign head stays"
        );
        let (key, editor) = &upgrade.library[0];
        assert_eq!(key, EDITOR);
        assert_eq!(editor[..old.len()], old[..], "every old byte");
        assert_eq!(editor[old.len()..], [0; 20], "four empty sets, no Recent");
        let editor = crate::editor::decode_editor(editor).expect("a schema-22 toolbox");
        assert_eq!((editor.tool, editor.library.len()), (Tool::Eraser, 2));
        assert_eq!(editor.sets, <[Vec<String>; 4]>::default());
        assert!(editor.recent_brushes.is_empty());
        assert_eq!(upgrade.library[1], (SCHEMA_KEY.to_vec(), encode_schema(22)));
        for broken in [&old[..old.len() - 1], &[old.as_slice(), &[0]].concat()[..]] {
            let upgrade = upgrade_21_to_22(&[], Some(broken));
            assert_eq!(upgrade.library.len(), 1, "a broken toolbox stays as it is");
        }
        assert_eq!(upgrade_21_to_22(&[], None).library.len(), 1, "none: none");
    }
}
