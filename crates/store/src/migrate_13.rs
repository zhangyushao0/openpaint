//! Schema 12 -> 13 (DECISIONS D57): edit records gain kind 3, a transform record without
//! pixels (`fills.rs`), which schema 12 readers took for damage. No existing record changes:
//! older transforms keep their patches (kind 1) and render as before, and heads are only marked
//! 13, so a schema-12 build reports a newer library rather than a damaged artwork.

use openpaint_app::ProjectId;

use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 12 -> 13, from every project's head as schema 12 stored it. A head not marked 12 is
/// left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_12_to_13(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 12, 13) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(13)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_are_marked_13_and_keep_every_other_byte() {
        let head = [&12u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![9, 0, 0, 0, 7]),
        ];
        let upgrade = upgrade_12_to_13(&heads);
        let marked = [&13u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        assert_eq!(upgrade.projects, [(head_key(ProjectId::new(1)), marked)]);
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(13))]);
    }
}
