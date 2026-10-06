//! Schema 10 -> 11 (DECISIONS D51): layers gain a mask and heads whether the active layer's
//! mask is the target. A pure-data step like those in `migrate.rs`: every layer of a head gets
//! "no mask" and the head "paint is the target", so nothing looks different after the upgrade.
//! Strokes, edits and the toolbox keep every byte.

use openpaint_app::ProjectId;

use crate::codec::{Reader, Writer};
use crate::migrate::Upgrade;
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 10 -> 11, from every project's head as schema 10 stored it. A head that is not a
/// whole schema-10 head is left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_10_to_11(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = head_10_to_11(head) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(11)));
    upgrade
}

/// A schema-10 head as schema 11 stores it: every byte kept, the schema 11, after each layer's
/// edit count "no mask" (u8 0), and after the groups "paint is the target" (u8 0). `None` when
/// it is not a whole schema-10 head.
pub(crate) fn head_10_to_11(head: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(head, "artwork head");
    let mut writer = Writer::with_capacity(head.len() + 64);
    (reader.u32().ok()? == 10).then_some(())?;
    writer.u32(11);
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
        for _ in 0..4 {
            writer.u8(reader.u8().ok()?);
        }
        writer.u64(reader.u64().ok()?);
        writer.u32(reader.u32().ok()?);
        writer.u32(reader.u32().ok()?);
        writer.u8(0);
    }
    let groups = reader.u32().ok()?;
    writer.u32(groups);
    for _ in 0..groups {
        writer.u64(reader.u64().ok()?);
        writer.str(&reader.str().ok()?);
        writer.u8(reader.u8().ok()?);
        writer.f32(reader.f32().ok()?);
    }
    reader.end().ok()?;
    writer.u8(0);
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::decode_head;
    use openpaint_core::BlendMode;

    /// A schema-10 head of two layers, "A" (3 strokes, 1 edit, Multiply, clipping, in group 7)
    /// and "Bé" (hidden, 2 edits), and group 7 "G" (hidden, opacity 0.4).
    fn head_10() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u32(10);
        writer.u32(64);
        writer.u32(48);
        for value in [2u64, 8, 9] {
            writer.u64(value);
        }
        writer.u32(2);
        for (id, name, visible, group, counts) in
            [(1u64, "A", 1u8, 7u64, [3u32, 1]), (2, "Bé", 0, 0, [0, 2])]
        {
            writer.u64(id);
            writer.str(name);
            writer.u8(visible);
            writer.f32(0.5);
            writer.u8(0);
            writer.u8(0);
            writer.u8(u8::from(id == 1));
            writer.u8(u8::from(id == 1));
            writer.u64(group);
            writer.u32(counts[0]);
            writer.u32(counts[1]);
        }
        writer.u32(1);
        writer.u64(7);
        writer.str("G");
        writer.u8(0);
        writer.f32(0.4);
        writer.finish()
    }

    #[test]
    fn layers_gain_no_mask_and_heads_paint_as_the_target() {
        let heads = [
            (ProjectId::new(1), head_10()),
            (ProjectId::new(2), vec![9, 0, 0, 0]),
        ];
        let upgrade = upgrade_10_to_11(&heads);
        assert_eq!(upgrade.projects.len(), 1, "a foreign head stays");
        let (key, head) = &upgrade.projects[0];
        assert_eq!(key, &head_key(ProjectId::new(1)));
        let head = crate::migrate::remarked(head, 11, 12).expect("marked 11");
        let head = crate::migrate::remarked(&head, 12, 13).expect("marked 12");
        let head = crate::migrate::remarked(&head, 13, 14).expect("marked 13");
        let head = crate::migrate::remarked(&head, 14, 15).expect("marked 14");
        let head = crate::migrate::remarked(&head, 15, 16).expect("marked 15");
        let head = decode_head(&head).expect("a schema-11 head");
        assert!(!head.on_mask);
        assert_eq!((head.width, head.active, head.next_stroke_id), (64, 2, 9));
        assert_eq!(head.groups.len(), 1);
        assert_eq!(
            (head.groups[0].name.as_str(), head.groups[0].visible),
            ("G", false)
        );
        let layers: Vec<_> = head
            .layers
            .iter()
            .map(|(layer, strokes, edits, masked)| {
                assert!(layer.mask.is_none());
                assert_eq!(*masked, [0, 0]);
                let props = (layer.clipping, layer.blend, layer.group);
                (layer.name.as_str(), props, *strokes, *edits)
            })
            .collect();
        let multiply = (true, BlendMode::Multiply, Some(7));
        let normal = (false, BlendMode::Normal, None);
        assert_eq!(layers, [("A", multiply, 3, 1), ("Bé", normal, 0, 2)]);
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(11))]);
        let broken = upgrade_10_to_11(&[(ProjectId::new(3), head_10()[..30].to_vec())]);
        assert!(broken.projects.is_empty(), "a broken head stays as it is");
    }
}
