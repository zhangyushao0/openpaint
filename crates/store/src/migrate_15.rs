//! Schema 14 -> 15 (DECISIONS D66): edit records gain kind 4, a layer merged down into another
//! (`nested.rs`), which schema 14 readers took for damage. No existing record changes; heads
//! are only marked 15, so a schema-14 build reports a newer library rather than a damaged
//! artwork.

use openpaint_app::ProjectId;

use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 14 -> 15, from every project's head as schema 14 stored it. A head not marked 14 is
/// left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_14_to_15(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 14, 15) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(15)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_are_marked_15_and_keep_every_other_byte() {
        let head = [&14u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![9, 0, 0, 0, 7]),
        ];
        let upgrade = upgrade_14_to_15(&heads);
        let marked = [&15u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        assert_eq!(upgrade.projects, [(head_key(ProjectId::new(1)), marked)]);
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(15))]);
    }
}
