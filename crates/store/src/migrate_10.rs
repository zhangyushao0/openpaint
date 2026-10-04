//! Schema 9 -> 10 (DECISIONS D50): layers gain Clipping Mask, a blend mode and a group, and
//! heads a group table. A pure-data step like those in `migrate.rs`: every layer of a head
//! becomes not clipping, Normal and in no group, and the head ends with no groups, so nothing
//! looks different after the upgrade. Strokes, edits and the toolbox keep every byte.

use openpaint_app::ProjectId;

use crate::codec::{Reader, Writer};
use crate::migrate::Upgrade;
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 9 -> 10, from every project's head as schema 9 stored it. A head that is not a whole
/// schema-9 head is left as it is, so opening its project reports what is wrong, as before.
pub(crate) fn upgrade_9_to_10(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = head_9_to_10(head) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(10)));
    upgrade
}

/// A schema-9 head as schema 10 stores it: every byte kept, the schema 10, after each layer's
/// Alpha Lock "not clipping" (0), "Normal" (0) and "no group" (u64 0), and no groups at the
/// end. `None` when it is not a whole schema-9 head.
fn head_9_to_10(head: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(head, "artwork head");
    let mut writer = Writer::with_capacity(head.len() + 64);
    (reader.u32().ok()? == 9).then_some(())?;
    writer.u32(10);
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
        writer.u8(0);
        writer.u8(0);
        writer.u64(0);
        writer.u32(reader.u32().ok()?);
        writer.u32(reader.u32().ok()?);
    }
    reader.end().ok()?;
    writer.u32(0);
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::decode_head;
    use openpaint_core::BlendMode;

    /// A schema-9 head of two layers: "A" (visible, opacity 0.5, locked, alpha locked, 3
    /// strokes, 1 edit) and "Bé" (hidden, 0 strokes, 2 edits).
    fn head_9() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u32(9);
        writer.u32(64);
        writer.u32(48);
        for value in [2u64, 3, 9] {
            writer.u64(value);
        }
        writer.u32(2);
        for (id, name, visible, opacity, counts) in [
            (1u64, "A", 1u8, 0.5f32, [3u32, 1]),
            (2, "Bé", 0, 1.0, [0, 2]),
        ] {
            writer.u64(id);
            writer.str(name);
            writer.u8(visible);
            writer.f32(opacity);
            writer.u8(1);
            writer.u8(u8::from(id == 1));
            writer.u32(counts[0]);
            writer.u32(counts[1]);
        }
        writer.finish()
    }

    #[test]
    fn layers_become_normal_unclipped_and_ungrouped_and_heads_gain_no_groups() {
        let heads = [
            (ProjectId::new(1), head_9()),
            (ProjectId::new(2), vec![8, 0, 0, 0]),
        ];
        let upgrade = upgrade_9_to_10(&heads);
        assert_eq!(upgrade.projects.len(), 1, "a foreign head stays");
        let (key, head) = &upgrade.projects[0];
        assert_eq!(key, &head_key(ProjectId::new(1)));
        let head = decode_head(head).expect("a schema-10 head");
        assert_eq!((head.width, head.active, head.next_stroke_id), (64, 2, 9));
        assert!(head.groups.is_empty());
        let layers: Vec<_> = head
            .layers
            .iter()
            .map(|(layer, strokes, edits)| {
                let props = (layer.clipping, layer.blend, layer.group);
                assert_eq!(props, (false, BlendMode::Normal, None));
                (
                    layer.name.as_str(),
                    layer.visible,
                    layer.alpha_locked,
                    *strokes,
                    *edits,
                )
            })
            .collect();
        assert_eq!(
            layers,
            [("A", true, true, 3, 1), ("Bé", false, false, 0, 2)]
        );
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(10))]);
        let broken = upgrade_9_to_10(&[(ProjectId::new(3), head_9()[..20].to_vec())]);
        assert!(broken.projects.is_empty(), "a broken head stays as it is");
    }
}
