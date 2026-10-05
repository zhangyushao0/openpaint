//! Schema 13 -> 14 (DECISIONS D59): adjustment records (edit kind 2) gain what they do 2, a
//! Gaussian blur, and 3, sharpening (`adjusts.rs`), which schema 13 readers took for damage.
//! No existing record changes; heads are only marked 14, so a schema-13 build reports a newer
//! library rather than a damaged artwork.

use openpaint_app::ProjectId;

use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 13 -> 14, from every project's head as schema 13 stored it. A head not marked 13 is
/// left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_13_to_14(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 13, 14) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(14)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_are_marked_14_and_keep_every_other_byte() {
        let head = [&13u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![9, 0, 0, 0, 7]),
        ];
        let upgrade = upgrade_13_to_14(&heads);
        let marked = [&14u32.to_le_bytes()[..], &[1, 2, 3, 4, 5]].concat();
        assert_eq!(upgrade.projects, [(head_key(ProjectId::new(1)), marked)]);
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(14))]);
    }
}
