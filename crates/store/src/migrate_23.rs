//! Schema 22 -> 23 (S54, DECISIONS D81): every head ends with the save id that wrote it, which
//! keys the display cache of the artwork's canvas. A schema-22 head gets 0, no save id, after
//! every old byte; the store writes a new random one into it when the artwork is next opened or
//! saved (a step is pure data, so it draws none). No other record changes.

use openpaint_app::ProjectId;

use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};

/// Schema 22 -> 23, from every project's head as schema 22 stored it. A head not marked 22 is
/// left as it is, so opening it reports what is wrong.
pub(crate) fn upgrade_22_to_23(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = head_22_to_23(head) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(23)));
    upgrade
}

/// A head marked 22, marked 23 with no save id (0) after its last byte.
pub(crate) fn head_22_to_23(head: &[u8]) -> Option<Vec<u8>> {
    let mut head = remarked(head, 22, 23)?;
    head.extend_from_slice(&0u64.to_le_bytes());
    Some(head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_gain_no_save_id_and_are_marked() {
        let head = [&22u32.to_le_bytes()[..], &[4, 5, 6]].concat();
        let heads = [
            (ProjectId::new(1), head),
            (ProjectId::new(2), vec![21, 0, 0, 0, 7]),
        ];
        let upgrade = upgrade_22_to_23(&heads);
        let marked = [&23u32.to_le_bytes()[..], &[4, 5, 6], &[0; 8]].concat();
        assert_eq!(
            upgrade.projects,
            [(head_key(ProjectId::new(1)), marked)],
            "a foreign head stays"
        );
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(23))]);
    }
}
