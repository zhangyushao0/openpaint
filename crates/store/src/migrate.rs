//! Schema migrations: one pure-data step per version (DECISIONS D09, D25, D27). A step takes the
//! records it needs as they were read and returns the records to write; the library writes them
//! in one atomic batch when it opens an older library, before anything else reads it.

use openpaint_app::{ProjectId, Title};
use openpaint_core::RecipeParts;

use crate::catalogue::{Entry, encode_entry};
use crate::codec::Writer;
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
        },
    );
    writer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::decode_stroke;
    use openpaint_core::BrushRecipe;

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
        let stroke = decode_stroke(&upgrade.projects[1].1).expect("a schema-3 stroke");
        assert_eq!((stroke.id, stroke.size, stroke.opacity), (9, 12.0, 0.5));
        assert_eq!(stroke.samples.len(), 1);
        assert_eq!(stroke.samples[0].time_ms, 7);
        let recipe = BrushRecipe::from_parts(&stroke.recipe).expect("valid");
        assert_eq!(recipe, BrushRecipe::plain(), "the look before recipes");
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(3))]);
    }
}
