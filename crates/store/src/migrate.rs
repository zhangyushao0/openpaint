//! Schema migrations: one pure-data step per version (DECISIONS D09, D25). A step takes the
//! records it needs as they were read and returns the records to write; the library writes them
//! in one atomic batch when it opens an older library, before anything else reads it.

use openpaint_app::{ProjectId, Title};

use crate::catalogue::{Entry, encode_entry};
use crate::records::{CLOCK, SCHEMA_KEY, encode_id, encode_schema, entry_key, head_key};

/// Records to write: `(key, value)` per keyspace.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Upgrade {
    pub(crate) library: Vec<(Vec<u8>, Vec<u8>)>,
    pub(crate) projects: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Schema 1 -> 2, from every project's head as schema 1 stored it. Each project gets a catalogue
/// entry titled "Untitled Artwork", touched in creation order (its id), so the newest is listed
/// first; the change counter starts after the newest; heads are marked schema 2 (their layout
/// did not change). `last` keeps naming the project saved last, which schema 1 reopened, so
/// the next start opens the same artwork. A head that is not a schema-1 head is left as it is:
/// it stays listed, and opening it reports what is wrong, as before.
pub(crate) fn upgrade_1_to_2(heads: &[(ProjectId, Vec<u8>)]) -> Upgrade {
    let mut upgrade = Upgrade::default();
    let mut clock = 0;
    for (id, head) in heads {
        if let Some(layout) = head.strip_prefix(&1u32.to_le_bytes()[..]) {
            let mut marked = encode_schema(2);
            marked.extend_from_slice(layout);
            upgrade.projects.push((head_key(*id), marked));
        }
        let entry = Entry {
            title: Title::untitled(),
            touched: id.get(),
        };
        upgrade.library.push((entry_key(*id), encode_entry(&entry)));
        clock = clock.max(id.get());
    }
    upgrade.library.push((CLOCK.to_vec(), encode_id(clock)));
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(2)));
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_are_marked_and_every_project_gets_an_entry() {
        let one = ProjectId::new(1);
        let three = ProjectId::new(3);
        let mut head = 1u32.to_le_bytes().to_vec();
        head.extend_from_slice(&[7, 8, 9]);
        let upgrade = upgrade_1_to_2(&[(one, head), (three, vec![5])]);
        assert_eq!(
            upgrade.projects,
            [(head_key(one), vec![2, 0, 0, 0, 7, 8, 9])],
            "an unreadable head stays as it is"
        );
        let untitled = |touched| {
            encode_entry(&Entry {
                title: Title::untitled(),
                touched,
            })
        };
        assert_eq!(
            upgrade.library,
            [
                (entry_key(one), untitled(1)),
                (entry_key(three), untitled(3)),
                (CLOCK.to_vec(), encode_id(3)),
                (SCHEMA_KEY.to_vec(), encode_schema(2)),
            ]
        );
    }
}
