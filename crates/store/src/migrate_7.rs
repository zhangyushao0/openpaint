//! Schema 6 -> 7 (DECISIONS D43): strokes gain their clip, the selection they were drawn
//! inside. A pure-data step like those in `migrate.rs`: no stroke before schema 7 was drawn
//! inside a selection, so every old stroke gets "no clip" and nothing looks different.

use openpaint_app::ProjectId;

use crate::codec::Reader;
use crate::migrate::Upgrade;
use crate::records::{SCHEMA_KEY, encode_schema, head_key};
use crate::strokes::read_stroke_6;

/// Schema 6 -> 7, from every project's head and every stroke record as schema 6 stored them.
/// Heads are marked schema 7 (their layout did not change); each stroke gets "no clip" (0)
/// after its samples. Records that are not whole schema-6 records are left as they are, so
/// opening their project reports what is wrong, as before.
pub(crate) fn upgrade_6_to_7(
    heads: &[(ProjectId, Vec<u8>)],
    strokes: &[(Vec<u8>, Vec<u8>)],
) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(rest) = head.strip_prefix(&6u32.to_le_bytes()[..]) {
            let marked = [&7u32.to_le_bytes()[..], rest].concat();
            upgrade.projects.push((head_key(*id), marked));
        }
    }
    for (key, value) in strokes {
        let mut reader = Reader::new(value, "stroke");
        let whole = read_stroke_6(&mut reader).is_ok() && reader.end().is_ok();
        if whole {
            upgrade
                .projects
                .push((key.clone(), [&value[..], &[0]].concat()));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(7)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Writer;
    use crate::records::{decode_stroke, write_recipe};
    use openpaint_core::BrushRecipe;

    /// A schema-6 stroke of the plain round brush: id 9, size 12, opacity 0.5, black, painting
    /// over, covering, two samples.
    fn stroke_6() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u64(9);
        for value in [12.0f32, 0.5, 0.0, 0.0, 0.0] {
            writer.f32(value);
        }
        writer.u8(0);
        write_recipe(&mut writer, &BrushRecipe::plain().to_parts());
        writer.u8(0);
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
    fn strokes_get_no_clip_and_keep_every_byte() {
        let old = stroke_6();
        let heads = [
            (ProjectId::new(1), [&6u32.to_le_bytes()[..], &[7]].concat()),
            (ProjectId::new(2), vec![5, 0, 0, 0]),
        ];
        let strokes = [
            (b"key".to_vec(), old.clone()),
            (b"cut".to_vec(), old[..old.len() - 1].to_vec()),
        ];
        let upgrade = upgrade_6_to_7(&heads, &strokes);
        assert_eq!(
            upgrade.projects.len(),
            2,
            "a cut stroke and a foreign head stay"
        );
        assert_eq!(
            upgrade.projects[0],
            (head_key(ProjectId::new(1)), vec![7, 0, 0, 0, 7])
        );
        let (key, new) = &upgrade.projects[1];
        assert_eq!(key, b"key");
        assert_eq!(new[..old.len()], old[..]);
        let stroke = decode_stroke(new).expect("a schema-7 stroke");
        assert_eq!((stroke.id, stroke.samples.len(), stroke.clip), (9, 2, None));
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(7))]);
    }
}
