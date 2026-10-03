//! Schema 7 -> 8 (DECISIONS D45): a layer's region edits are fills or transforms, so each edit
//! record starts with its kind. A pure-data step like those in `migrate.rs`: every record before
//! schema 8 is a fill, so each gets the kind "fill" (0) in front and nothing looks different.

use openpaint_app::ProjectId;

use crate::codec::Reader;
use crate::fills::read_fill;
use crate::migrate::Upgrade;
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 7 -> 8, from every project's head and every fill record as schema 7 stored them.
/// Heads are marked schema 8 (their layout did not change: a layer's fill count is now its edit
/// count); each fill gets the kind 0 before its bytes. Records that are not whole schema-7
/// records are left as they are, so opening their project reports what is wrong, as before.
pub(crate) fn upgrade_7_to_8(
    heads: &[(ProjectId, Vec<u8>)],
    fills: &[(Vec<u8>, Vec<u8>)],
) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(rest) = head.strip_prefix(&7u32.to_le_bytes()[..]) {
            let marked = [&8u32.to_le_bytes()[..], rest].concat();
            upgrade.projects.push((head_key(*id), marked));
        }
    }
    for (key, value) in fills {
        let mut reader = Reader::new(value, "fill");
        let whole = read_fill(&mut reader).is_ok() && reader.end().is_ok();
        if whole {
            upgrade
                .projects
                .push((key.clone(), [&[0], &value[..]].concat()));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(8)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Writer;
    use crate::fills::decode_edit;
    use openpaint_core::EditParts;

    /// A schema-7 fill record: after 2, a colour, no Alpha Lock, a 3 x 1 mask of one run.
    fn fill_7() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u32(2);
        for value in [0.25f32, 0.5, 1.0] {
            writer.f32(value);
        }
        writer.u8(0);
        for value in [4u32, 5, 3, 1, 2] {
            writer.u32(value);
        }
        writer.varint(0);
        writer.varint(3);
        writer.finish()
    }

    #[test]
    fn fills_get_their_kind_and_keep_every_byte() {
        let old = fill_7();
        let heads = [
            (ProjectId::new(1), [&7u32.to_le_bytes()[..], &[9]].concat()),
            (ProjectId::new(2), vec![6, 0, 0, 0]),
        ];
        let fills = [
            (b"key".to_vec(), old.clone()),
            (b"cut".to_vec(), old[..old.len() - 1].to_vec()),
        ];
        let upgrade = upgrade_7_to_8(&heads, &fills);
        assert_eq!(
            upgrade.projects.len(),
            2,
            "a cut fill and a foreign head stay"
        );
        assert_eq!(
            upgrade.projects[0],
            (head_key(ProjectId::new(1)), vec![8, 0, 0, 0, 9])
        );
        let (key, new) = &upgrade.projects[1];
        assert_eq!((key.as_slice(), new[0]), (&b"key"[..], 0));
        assert_eq!(new[1..], old[..]);
        let EditParts::Fill(fill) = decode_edit(new).expect("a schema-8 fill") else {
            panic!("a fill");
        };
        assert_eq!((fill.after, fill.runs), (2, vec![0, 3]));
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(8))]);
    }
}
