//! Schema 11 -> 12 (DECISIONS D54): edit records gain kind 2, an adjustment (`adjusts.rs`),
//! which schema 11 readers took for damage. No existing record changes: heads are only marked
//! 12, so a schema-11 build reports a newer library rather than a damaged artwork. Strokes,
//! edits and the toolbox keep every byte.

use openpaint_app::ProjectId;

use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 11 -> 12, from every project's head as schema 11 stored it. A head not marked 11 is
/// left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_11_to_12(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 11, 12) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(12)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_are_marked_12_and_keep_every_other_byte() {
        let head = [&11u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![9, 0, 0, 0, 7]),
        ];
        let upgrade = upgrade_11_to_12(&heads);
        let marked = [&12u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        assert_eq!(upgrade.projects, [(head_key(ProjectId::new(1)), marked)]);
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(12))]);
    }
}
