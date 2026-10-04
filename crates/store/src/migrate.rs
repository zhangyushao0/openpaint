//! Schema migrations: one pure-data step per version (DECISIONS D09, D25, D28, D29, D36). A step takes the
//! records it needs as they were read and returns the records to write; the library writes them
//! in one atomic batch when it opens an older library, before anything else reads it.

use openpaint_app::{BrushChoice, ProjectId, Title};
use openpaint_core::RecipeParts;

use crate::catalogue::{Entry, encode_entry};
use crate::codec::{Reader, Writer};
use crate::editor::{EDITOR, write_choice, write_color};
use crate::records::{
    CLOCK, SCHEMA_KEY, STROKE_STYLE_BYTES, encode_id, encode_schema, entry_key, head_key,
    write_recipe,
};

/// Records to write: `(key, value)` per keyspace.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Upgrade {
    pub(crate) library: Vec<(Vec<u8>, Vec<u8>)>,
    pub(crate) projects: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Schema 1 -> 2, from every project's head as schema 1 stored it. Each project gets a catalogue
/// entry titled "Untitled Artwork", touched in creation order (its id), so the newest is listed
/// first; the change counter starts after the newest; heads are marked schema 2 (their layout
/// did not change). `last` keeps naming the project saved last, which schema 1 reopened, so
/// the next start opens the same artwork. A head that is not a schema-1 head is left as it is:
/// it stays listed, and opening it reports what is wrong, as before.
pub(crate) fn upgrade_1_to_2(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    let mut clock = 0;
    for (id, head) in heads {
        if let Some(marked) = remarked(head, 1, 2) {
            upgrade.projects.push((head_key(*id), marked));
        }
        let entry = Entry {
            title: Title::untitled(),
            touched: id.get(),
        };
        upgrade.library.push((entry_key(*id), encode_entry(&entry)));
        clock = clock.max(id.get());
    }
    upgrade.library.push((CLOCK.to_vec(), encode_id(clock)));
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(2)));
    upgrade
}

/// Schema 2 -> 3, from every project's head and every stroke record as schema 2 stored them.
/// Each stroke gets the recipe of the plain round brush, the only brush there was, right after
/// its colour, so it keeps its look exactly; heads are marked schema 3 (their layout did not
/// change). Records that are not schema-2 records are left as they are, so opening their
/// project reports what is wrong, as before. The toolbox record starts absent: defaults.
pub(crate) fn upgrade_2_to_3(
    heads: &[(ProjectId, Vec<u8>)],
    strokes: &[(Vec<u8>, Vec<u8>)],
) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(marked) = remarked(head, 2, 3) {
            upgrade.projects.push((head_key(*id), marked));
        }
    }
    let recipe = legacy_recipe();
    for (key, value) in strokes {
        if value.len() >= STROKE_STYLE_BYTES {
            let (style, samples) = value.split_at(STROKE_STYLE_BYTES);
            let stroke = [style, &recipe, samples].concat();
            upgrade.projects.push((key.clone(), stroke));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(3)));
    upgrade
}

/// Schema 3 -> 4, from every project's head, every stroke record and the toolbox record as
/// schema 3 stored them. Each layer of a head gets Alpha Lock off after its lock; each stroke
/// gets the blend "paint over" after its colour, so it keeps its look exactly; the toolbox keeps
/// the brush tool's choice, with the brush tool current, the eraser on its default brush, black
/// and no recent colours. Records that are not schema-3 records are left as they are.
pub(crate) fn upgrade_3_to_4(
    heads: &[(ProjectId, Vec<u8>)],
    strokes: &[(Vec<u8>, Vec<u8>)],
    editor: Option<&[u8]>,
) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = head_3_to_4(head) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    for (key, value) in strokes {
        if value.len() >= STROKE_STYLE_BYTES {
            let (style, rest) = value.split_at(STROKE_STYLE_BYTES);
            upgrade
                .projects
                .push((key.clone(), [style, &[0], rest].concat()));
        }
    }
    if let Some(editor) = editor {
        let mut writer = Writer::default();
        writer.u8(0);
        writer.bytes(editor);
        let default = BrushChoice {
            brush: String::new(),
            brushes: Vec::new(),
        };
        write_choice(&mut writer, &default);
        write_color(&mut writer, [0.0; 3]);
        writer.len(0);
        upgrade.library.push((EDITOR.to_vec(), writer.finish()));
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(4)));
    upgrade
}

/// A schema-3 head as schema 4 stores it: every byte kept, the schema 4, and after each layer's
/// lock its Alpha Lock, off. `None` when it is not a whole schema-3 head.
fn head_3_to_4(head: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(head, "artwork head");
    let mut writer = Writer::with_capacity(head.len() + 64);
    (reader.u32().ok()? == 3).then_some(())?;
    writer.u32(4);
    for _ in 0..2 {
        writer.u32(reader.u32().ok()?);
    }
    for _ in 0..3 {
        writer.u64(reader.u64().ok()?);
    }
    let count = reader.u32().ok()?;
    writer.u32(count);
    for _ in 0..count {
        writer.u64(reader.u64().ok()?);
        writer.str(&reader.str().ok()?);
        writer.u8(reader.u8().ok()?);
        writer.f32(reader.f32().ok()?);
        writer.u8(reader.u8().ok()?);
        writer.u8(0);
        writer.u32(reader.u32().ok()?);
    }
    reader.end().ok()?;
    Some(writer.finish())
}

/// Schema 4 -> 5, from every project's head as schema 4 stored it. Each layer of a head gets a
/// fill count of 0 after its stroke count: no artwork had fills. Strokes and the toolbox keep
/// their bytes. A head that is not a whole schema-4 head is left as it is.
pub(crate) fn upgrade_4_to_5(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = head_4_to_5(head) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(5)));
    upgrade
}

/// A schema-4 head as schema 5 stores it: every byte kept, the schema 5, and after each layer's
/// stroke count its fill count, 0. `None` when it is not a whole schema-4 head.
fn head_4_to_5(head: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(head, "artwork head");
    let mut writer = Writer::with_capacity(head.len() + 64);
    (reader.u32().ok()? == 4).then_some(())?;
    writer.u32(5);
    for _ in 0..2 {
        writer.u32(reader.u32().ok()?);
    }
    for _ in 0..3 {
        writer.u64(reader.u64().ok()?);
    }
    let count = reader.u32().ok()?;
    writer.u32(count);
    for _ in 0..count {
        writer.u64(reader.u64().ok()?);
        writer.str(&reader.str().ok()?);
        writer.u8(reader.u8().ok()?);
        writer.f32(reader.f32().ok()?);
        writer.u8(reader.u8().ok()?);
        writer.u8(reader.u8().ok()?);
        writer.u32(reader.u32().ok()?);
        writer.u32(0);
    }
    reader.end().ok()?;
    Some(writer.finish())
}

/// `head` marked with schema `to` instead of `from`; `None` when it is not marked `from`.
fn remarked(head: &[u8], from: u32, to: u32) -> Option<Vec<u8>> {
    let layout = head.strip_prefix(&from.to_le_bytes()[..])?;
    let mut marked = encode_schema(to);
    marked.extend_from_slice(layout);
    Some(marked)
}

/// The brush of every stroke before schema 3, encoded as schema 3 stores a recipe: a hard
/// edge, the diameter in proportion to pressure, full strength, no grain. Frozen here, apart
/// from the built-in brushes, so it keeps describing the old look whatever they become.
fn legacy_recipe() -> Vec<u8> {
    let mut writer = Writer::default();
    write_recipe(
        &mut writer,
        &RecipeParts {
            softness: 0.0,
            size_pressure: vec![[0.0, 0.0], [1.0, 1.0]],
            size_speed: vec![[0.0, 1.0]],
            opacity_pressure: vec![[0.0, 1.0]],
            opacity_speed: vec![[0.0, 1.0]],
            grain_strength: 0.0,
            grain_scale: 1,
            // Not written: schema 3 had no deposit; the 5 -> 6 step adds "cover".
            deposit: None,
        },
    );
    writer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::decode_editor;
    use crate::migrate_6::upgrade_5_to_6;
    use crate::migrate_7::upgrade_6_to_7;
    use crate::migrate_8::upgrade_7_to_8;
    use crate::migrate_9::upgrade_8_to_9;
    use crate::records::{decode_head, decode_stroke};
    use openpaint_app::Tool;
    use openpaint_core::{BrushRecipe, StrokeBlend};

    #[test]
    fn heads_are_marked_and_every_project_gets_an_entry() {
        let one = ProjectId::new(1);
        let three = ProjectId::new(3);
        let mut head = 1u32.to_le_bytes().to_vec();
        head.extend_from_slice(&[7, 8, 9]);
        let upgrade = upgrade_1_to_2(&[(one, head), (three, vec![5])]);
        assert_eq!(
            upgrade.projects,
            [(head_key(one), vec![2, 0, 0, 0, 7, 8, 9])],
            "an unreadable head stays as it is"
        );
        let untitled = |touched| {
            encode_entry(&Entry {
                title: Title::untitled(),
                touched,
            })
        };
        assert_eq!(
            upgrade.library,
            [
                (entry_key(one), untitled(1)),
                (entry_key(three), untitled(3)),
                (CLOCK.to_vec(), encode_id(3)),
                (SCHEMA_KEY.to_vec(), encode_schema(2)),
            ]
        );
    }

    #[test]
    fn strokes_get_the_plain_round_brush_and_keep_everything_else() {
        // A schema-2 stroke: id 9, size 12, opacity 0.5, black, one sample.
        let mut old = Vec::new();
        old.extend_from_slice(&9u64.to_le_bytes());
        for value in [12.0f32, 0.5, 0.0, 0.0, 0.0] {
            old.extend_from_slice(&value.to_le_bytes());
        }
        old.extend_from_slice(&1u32.to_le_bytes());
        for value in [3.0f32, 4.0, 0.25, 0.0, 0.0] {
            old.extend_from_slice(&value.to_le_bytes());
        }
        old.extend_from_slice(&7u32.to_le_bytes());
        let head = [2u32.to_le_bytes().to_vec(), vec![1, 2]].concat();
        let id = ProjectId::new(4);
        let strokes = [(b"key".to_vec(), old), (b"short".to_vec(), vec![1, 2])];
        let upgrade = upgrade_2_to_3(&[(id, head)], &strokes);
        assert_eq!(upgrade.projects[0], (head_key(id), vec![3, 0, 0, 0, 1, 2]));
        assert_eq!(upgrade.projects.len(), 2, "the short record stays as it is");
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(3))]);
        let then = upgrade_3_to_4(&[], &upgrade.projects[1..], None);
        let six = upgrade_5_to_6(&[], &then.projects, None);
        let seven = upgrade_6_to_7(&[], &six.projects);
        let stroke = decode_stroke(&seven.projects[0].1).expect("a schema-4 stroke, upgraded");
        assert_eq!((stroke.id, stroke.size, stroke.opacity), (9, 12.0, 0.5));
        assert_eq!(stroke.blend, StrokeBlend::Over);
        assert_eq!(stroke.samples.len(), 1);
        assert_eq!(stroke.samples[0].time_ms, 7);
        let recipe = BrushRecipe::from_parts(&stroke.recipe).expect("valid");
        assert_eq!(recipe, BrushRecipe::plain(), "the look before recipes");
    }

    /// A schema-3 head of two layers: "A" (visible, opacity 0.5, locked, 3 strokes) and "Bé".
    fn head_3() -> Vec<u8> {
        let mut writer = Writer::default();
        for word in [3u32, 64, 48] {
            writer.u32(word);
        }
        for id in [2u64, 3, 9] {
            writer.u64(id);
        }
        writer.u32(2);
        for (id, name, locked, strokes) in [(1u64, "A", 1u8, 3u32), (2, "Bé", 0, 0)] {
            writer.u64(id);
            writer.str(name);
            writer.u8(1);
            writer.f32(0.5);
            writer.u8(locked);
            writer.u32(strokes);
        }
        writer.finish()
    }

    #[test]
    fn layers_get_alpha_lock_off_and_the_toolbox_the_eraser_and_black() {
        let id = ProjectId::new(2);
        // Schema 3's toolbox: the pencil, one setting of key, size and opacity.
        let mut old_editor = Writer::default();
        old_editor.str("pencil");
        old_editor.u32(1);
        old_editor.str("pencil");
        old_editor.f32(4.0);
        old_editor.f32(0.5);
        let pencil = BrushChoice {
            brush: "pencil".into(),
            brushes: vec![openpaint_app::BrushSetting {
                key: "pencil".into(),
                size: 4.0,
                opacity: 0.5,
                streamline: 0.0,
            }],
        };
        let heads = [(id, head_3()), (ProjectId::new(5), vec![3, 0, 0, 0, 1])];
        let upgrade = upgrade_3_to_4(&heads, &[], Some(&old_editor.finish()));
        assert_eq!(upgrade.projects.len(), 1, "a broken head stays as it is");
        let then = upgrade_4_to_5(&[(id, upgrade.projects[0].1.clone()), heads[1].clone()]);
        assert_eq!(then.projects.len(), 1, "a broken head stays as it is");
        assert_eq!(then.library, [(SCHEMA_KEY.to_vec(), encode_schema(5))]);
        let six = upgrade_5_to_6(&[(id, then.projects[0].1.clone())], &[], None);
        let seven = upgrade_6_to_7(&[(id, six.projects[0].1.clone())], &[]);
        let eight = upgrade_7_to_8(&[(id, seven.projects[0].1.clone())], &[]);
        let nine = upgrade_8_to_9(&[(id, eight.projects[0].1.clone())], None);
        let head = decode_head(&nine.projects[0].1).expect("a schema-5 head, upgraded");
        let layers: Vec<_> = head
            .layers
            .iter()
            .map(|(layer, strokes, fills)| {
                (
                    layer.name.as_str(),
                    layer.locked,
                    layer.alpha_locked,
                    *strokes,
                    *fills,
                )
            })
            .collect();
        assert_eq!(
            layers,
            [("A", true, false, 3, 0), ("Bé", false, false, 0, 0)]
        );
        assert_eq!((head.width, head.next_stroke_id), (64, 9));
        let (key, editor) = &upgrade.library[0];
        assert_eq!(key, EDITOR);
        let six = upgrade_5_to_6(&[], &[], Some(editor));
        let nine = upgrade_8_to_9(&[], Some(&six.library[0].1));
        let editor = decode_editor(&nine.library[0].1).expect("a schema-4 toolbox, upgraded");
        assert_eq!((editor.tool, &editor.paint), (Tool::Brush, &pencil));
        assert_eq!(editor.erase.brush, "", "the default brush");
        assert_eq!((editor.color, editor.recent.len()), ([0.0; 3], 0));
        assert_eq!(upgrade.library[1], (SCHEMA_KEY.to_vec(), encode_schema(4)));
    }
}
