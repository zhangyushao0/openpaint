//! Schema 15 -> 16 (S40): edit records gain kind 5, pixels cleared (a cut, or what a copy
//! leaves outside its selection, `fills.rs`), which schema 15 readers took for damage. No
//! existing record changes; heads are only marked 16, so a schema-15 build reports a newer
//! library rather than a damaged artwork.

use openpaint_app::ProjectId;

use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 15 -> 16, from every project's head as schema 15 stored it. A head not marked 15 is
/// left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_15_to_16(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 15, 16) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(16)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_are_marked_16_and_keep_every_other_byte() {
        let head = [&15u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![9, 0, 0, 0, 7]),
        ];
        let upgrade = upgrade_15_to_16(&heads);
        let marked = [&16u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        assert_eq!(upgrade.projects, [(head_key(ProjectId::new(1)), marked)]);
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(16))]);
    }
}
