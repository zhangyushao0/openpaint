//! Schema 8 -> 9 (DECISIONS D46): strokes may smudge and the toolbox has a third tool. A
//! pure-data step like those in `migrate.rs`: no stroke before schema 9 smudges, so stroke
//! records keep every byte (their blend is 0, 1 or 2 as before); the toolbox gets the smudge's
//! choice after the eraser's, empty (its default brush, every brush at its defaults), so nothing
//! looks or draws differently after the upgrade.

use openpaint_app::ProjectId;

use crate::codec::{Reader, Writer};
use crate::editor::{EDITOR, read_choice};
use crate::migrate::Upgrade;
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 8 -> 9, from every project's head and the toolbox record as schema 8 stored them.
/// Heads are marked schema 9 (their layout did not change). Records that are not whole schema-8
/// records are left as they are, so opening their project reports what is wrong, as before.
pub(crate) fn upgrade_8_to_9(heads: &[(ProjectId, Vec<u8>)], editor: Option<&[u8]>) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(rest) = head.strip_prefix(&8u32.to_le_bytes()[..]) {
            let marked = [&9u32.to_le_bytes()[..], rest].concat();
            upgrade.projects.push((head_key(*id), marked));
        }
    }
    if let Some(editor) = editor.and_then(editor_8_to_9) {
        upgrade.library.push((EDITOR.to_vec(), editor));
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(9)));
    upgrade
}

/// A schema-8 toolbox with an empty smudge choice after the eraser's. `None` when it is not a
/// whole schema-8 toolbox up to the colour.
fn editor_8_to_9(editor: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(editor, "editor state");
    let tool = reader.u8().ok()?;
    (tool <= 1).then_some(())?;
    read_choice(&mut reader).ok()?;
    read_choice(&mut reader).ok()?;
    let rest = reader.rest();
    let choices = &editor[..editor.len() - rest.len()];
    let mut writer = Writer::with_capacity(editor.len() + 8);
    writer.bytes(choices);
    writer.str("");
    writer.u32(0);
    writer.bytes(rest);
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_app::{BrushChoice, BrushSetting, Tool};

    /// A schema-8 toolbox: the eraser current, a pencil setting, the default eraser brush, a
    /// colour and one recent colour.
    fn editor_8() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u8(1);
        writer.str("pencil");
        writer.u32(1);
        writer.str("pencil");
        for value in [4.0f32, 0.5, 0.25] {
            writer.f32(value);
        }
        writer.str("");
        writer.u32(0);
        for value in [0.25f32, 0.5, 1.0] {
            writer.f32(value);
        }
        writer.u32(1);
        for value in [0.25f32, 0.5, 1.0] {
            writer.f32(value);
        }
        writer.finish()
    }

    #[test]
    fn the_toolbox_gains_a_default_smudge_and_heads_are_marked() {
        let heads = [
            (ProjectId::new(1), [&8u32.to_le_bytes()[..], &[7]].concat()),
            (ProjectId::new(2), vec![6, 0, 0, 0]),
        ];
        let old = editor_8();
        let upgrade = upgrade_8_to_9(&heads, Some(&old));
        assert_eq!(
            upgrade.projects,
            [(head_key(ProjectId::new(1)), vec![9, 0, 0, 0, 7])],
            "a foreign head stays"
        );
        let (key, editor) = &upgrade.library[0];
        assert_eq!(key, EDITOR);
        let editor = crate::migrate_17::upgraded_toolbox(editor);
        let pencil = BrushSetting {
            key: "pencil".into(),
            size: 4.0,
            opacity: 0.5,
            streamline: 0.25,
        };
        assert_eq!(editor.tool, Tool::Eraser);
        assert_eq!(editor.paint.brushes, [pencil]);
        let empty = BrushChoice {
            brush: String::new(),
            brushes: Vec::new(),
        };
        assert_eq!((&editor.erase, &editor.smudge), (&empty, &empty));
        assert_eq!((editor.color, editor.recent.len()), ([0.25, 0.5, 1.0], 1));
        assert_eq!(upgrade.library[1], (SCHEMA_KEY.to_vec(), encode_schema(9)));
        let broken = upgrade_8_to_9(&[], Some(&old[..5]));
        assert_eq!(broken.library.len(), 1, "a broken toolbox stays as it is");
    }
}
