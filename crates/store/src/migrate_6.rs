//! Schema 5 -> 6 (DECISIONS D38, D39): recipes gain their deposit and the toolbox its StreamLine
//! per brush. A pure-data step like those in `migrate.rs`: every old stroke covers, as every
//! stroke did before, and every brush starts with StreamLine off, so nothing looks or draws
//! differently after the upgrade.

use openpaint_app::ProjectId;

use crate::codec::{Reader, Writer};
use crate::editor::EDITOR;
use crate::migrate::Upgrade;
use crate::records::{SCHEMA_KEY, encode_schema, head_key, read_curve};

/// Schema 5 -> 6, from every project's head, every stroke record and the toolbox record as
/// schema 5 stored them. Heads are marked schema 6 (their layout did not change); each stroke
/// gets the deposit "cover" right after its recipe's grain; each brush of the toolbox gets
/// StreamLine 0 after its opacity. Records that are not whole schema-5 records are left as
/// they are, so opening their project reports what is wrong, as before.
pub(crate) fn upgrade_5_to_6(
    heads: &[(ProjectId, Vec<u8>)],
    strokes: &[(Vec<u8>, Vec<u8>)],
    editor: Option<&[u8]>,
) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(rest) = head.strip_prefix(&5u32.to_le_bytes()[..]) {
            let marked = [&6u32.to_le_bytes()[..], rest].concat();
            upgrade.projects.push((head_key(*id), marked));
        }
    }
    for (key, value) in strokes {
        if let Some(stroke) = stroke_5_to_6(value) {
            upgrade.projects.push((key.clone(), stroke));
        }
    }
    if let Some(editor) = editor.and_then(editor_5_to_6) {
        upgrade.library.push((EDITOR.to_vec(), editor));
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(6)));
    upgrade
}

/// A schema-5 stroke with the deposit "cover" (0) after its recipe's grain. `None` when the
/// record does not hold a whole schema-5 style and recipe.
fn stroke_5_to_6(stroke: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(stroke, "stroke");
    // Id, size, opacity, colour, blend, softness.
    reader.u64().ok()?;
    for _ in 0..5 {
        reader.f32().ok()?;
    }
    reader.u8().ok()?;
    reader.f32().ok()?;
    for _ in 0..4 {
        read_curve(&mut reader).ok()?;
    }
    reader.f32().ok()?;
    reader.u32().ok()?;
    let samples = reader.rest();
    let recipe_end = stroke.len() - samples.len();
    Some([&stroke[..recipe_end], &[0], samples].concat())
}

/// A schema-5 toolbox with StreamLine 0 after every brush's opacity. `None` when it is not a
/// whole schema-5 toolbox.
fn editor_5_to_6(editor: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(editor, "editor state");
    let mut writer = Writer::with_capacity(editor.len() + 64);
    writer.u8(reader.u8().ok()?);
    for _ in 0..2 {
        writer.str(&reader.str().ok()?);
        let count = reader.u32().ok()?;
        writer.u32(count);
        for _ in 0..count {
            writer.str(&reader.str().ok()?);
            writer.f32(reader.f32().ok()?);
            writer.f32(reader.f32().ok()?);
            writer.f32(0.0);
        }
    }
    writer.bytes(reader.rest());
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::decode_editor;
    use crate::records::{STROKE_STYLE_BYTES, write_recipe};
    use crate::strokes::read_stroke_6;
    use openpaint_app::{BrushChoice, BrushSetting, Tool};
    use openpaint_core::{BrushRecipe, Deposit};

    /// A schema-5 stroke of the plain round brush: id 9, size 12, opacity 0.5, black, painting
    /// over, two samples.
    fn stroke_5() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u64(9);
        for value in [12.0f32, 0.5, 0.0, 0.0, 0.0] {
            writer.f32(value);
        }
        writer.u8(0);
        write_recipe(&mut writer, &BrushRecipe::plain().to_parts());
        writer.u32(2);
        for (x, time) in [(3.0f32, 0u32), (9.0, 8)] {
            for value in [x, 4.0, 0.25, 0.5, -1.0] {
                writer.f32(value);
            }
            writer.u32(time);
        }
        writer.finish()
    }

    #[test]
    fn strokes_cover_and_keep_every_byte() {
        let old = stroke_5();
        let heads = [
            (
                ProjectId::new(1),
                [&5u32.to_le_bytes()[..], &[7, 8]].concat(),
            ),
            (ProjectId::new(2), vec![4, 0, 0, 0]),
        ];
        let strokes = [
            (b"key".to_vec(), old.clone()),
            (b"cut".to_vec(), old[..40].to_vec()),
        ];
        let upgrade = upgrade_5_to_6(&heads, &strokes, None);
        assert_eq!(
            upgrade.projects.len(),
            2,
            "a cut stroke and a foreign head stay"
        );
        assert_eq!(
            upgrade.projects[0],
            (head_key(ProjectId::new(1)), vec![6, 0, 0, 0, 7, 8])
        );
        let (key, new) = &upgrade.projects[1];
        assert_eq!(key, b"key");
        assert_eq!(new.len(), old.len() + 1);
        let mut reader = Reader::new(new, "stroke");
        let stroke = read_stroke_6(&mut reader).expect("a schema-6 stroke");
        reader.end().expect("whole");
        let recipe = BrushRecipe::from_parts(&stroke.recipe).expect("valid");
        assert_eq!(recipe, BrushRecipe::plain());
        assert_eq!(*recipe.deposit(), Deposit::Cover);
        assert_eq!(stroke.samples.len(), 2);
        assert_eq!(new[..STROKE_STYLE_BYTES], old[..STROKE_STYLE_BYTES]);
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(6))]);
    }

    #[test]
    fn every_brush_of_the_toolbox_starts_with_streamline_off() {
        // A schema-5 toolbox: the eraser current, a pencil setting, the default eraser brush,
        // a colour and one recent colour.
        let mut writer = Writer::default();
        writer.u8(1);
        writer.str("pencil");
        writer.u32(1);
        writer.str("pencil");
        writer.f32(4.0);
        writer.f32(0.5);
        writer.str("");
        writer.u32(0);
        for value in [0.25f32, 0.5, 1.0] {
            writer.f32(value);
        }
        writer.u32(1);
        for value in [0.25f32, 0.5, 1.0] {
            writer.f32(value);
        }
        let old = writer.finish();
        let upgrade = upgrade_5_to_6(&[], &[], Some(&old));
        let (key, editor) = &upgrade.library[0];
        assert_eq!(key, EDITOR);
        let editor = decode_editor(editor).expect("a schema-6 toolbox");
        let pencil = BrushSetting {
            key: "pencil".into(),
            size: 4.0,
            opacity: 0.5,
            streamline: 0.0,
        };
        assert_eq!(editor.tool, Tool::Eraser);
        assert_eq!(
            editor.paint,
            BrushChoice {
                brush: "pencil".into(),
                brushes: vec![pencil],
            }
        );
        assert_eq!((editor.color, editor.recent.len()), ([0.25, 0.5, 1.0], 1));
        let broken = upgrade_5_to_6(&[], &[], Some(&old[..5]));
        assert_eq!(broken.library.len(), 1, "a broken toolbox stays as it is");
    }
}
