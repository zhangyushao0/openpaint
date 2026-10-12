//! S50: an upgrade that fails on one artwork, over copies of the schema-18 sample: a record cut
//! short, a wrong tag, a step that panics. That artwork is kept as it was and listed as needing
//! repair; the others upgrade and open.

use openpaint_app::{PortError, ProjectId, ProjectStore};

use super::{Dump, artwork_of, library_of, records, sample};
use crate::Library;
use crate::records::{LIBRARY, NEXT, PROJECTS, SCHEMA_KEY, encode_id, entry_key};
use crate::steps::hook;

/// The schema-18 sample's project copied as projects 2, 3 and 4 (titles too).
fn four_projects() -> Dump {
    let one = sample(18);
    let mut out = one.clone();
    for id in 2..=4u64 {
        for (keyspace, key, value) in &one {
            let copied = match *keyspace {
                PROJECTS if key.starts_with(&1u64.to_be_bytes()) => {
                    [&id.to_be_bytes()[..], &key[8..]].concat()
                }
                LIBRARY if *key == entry_key(ProjectId::new(1)) => entry_key(ProjectId::new(id)),
                _ => continue,
            };
            out.push((keyspace, copied, value.clone()));
        }
    }
    let next = out.iter_mut().find(|(_, key, _)| key == NEXT);
    next.expect("the next project id").2 = encode_id(5);
    out
}

/// The first stroke record of project `id`.
fn first_stroke(records: &mut Dump, id: u64) -> &mut Vec<u8> {
    let stroke = records.iter_mut().find(|(keyspace, key, _)| {
        *keyspace == PROJECTS && key.starts_with(&id.to_be_bytes()) && key.get(8) == Some(&b's')
    });
    &mut stroke.expect("a stroke record").2
}

/// Project `id`'s records.
fn of_project(records: &Dump, id: u64) -> Dump {
    let prefix = id.to_be_bytes();
    let mine = |(keyspace, key, _): &&(&str, Vec<u8>, Vec<u8>)| {
        *keyspace == PROJECTS && key.starts_with(&prefix)
    };
    records.iter().filter(mine).cloned().collect()
}

#[test]
fn an_upgrade_failing_on_one_artwork_keeps_it_as_it_was_and_upgrades_the_others() {
    let control_dir = library_of("control", &four_projects());
    let mut control = Library::at(&control_dir);
    let expected = artwork_of(&mut control, 1).to_parts();
    let toolbox = control.editor().expect("the toolbox");
    drop(control);

    let mut stored = four_projects();
    // Project 2: a stroke record cut short; project 3: a stroke whose blend tag is unknown.
    first_stroke(&mut stored, 2).pop();
    first_stroke(&mut stored, 3)[28] = 9;
    let dir = library_of("broken", &stored);
    hook::fail_step(Some(ProjectId::new(4)));
    let mut library = Library::at(&dir);
    let listed = library.list().expect("the library opens");
    hook::fail_step(None);
    let repair: Vec<(u64, bool)> = listed
        .iter()
        .map(|item| (item.id.get(), item.needs_repair))
        .collect();
    let mut repair = repair;
    repair.sort_unstable();
    assert_eq!(repair, [(1, false), (2, true), (3, true), (4, true)]);
    assert_eq!(artwork_of(&mut library, 1).to_parts(), expected);
    assert_eq!(library.editor().expect("the toolbox"), toolbox);
    for id in 2..=4 {
        let refused = library.load(ProjectId::new(id));
        assert_eq!(refused, Err(PortError::NotUpgraded { schema: 18 }), "{id}");
    }
    // Its bytes out as a file of its own schema: the head, strokes, edits and thumbnail.
    let file = library.artwork_file(ProjectId::new(3)).expect("a file");
    assert_eq!(file[10..14], 18u32.to_le_bytes(), "the file's schema");
    let refused = library.import_file(&file);
    assert!(
        matches!(refused, Err(PortError::Corrupt { .. })),
        "{refused:?}"
    );
    // A step failing on a file's records (held as project 1) makes it damaged; the panicking
    // step was the test's: the file opens once the step works.
    let file = library.artwork_file(ProjectId::new(4)).expect("a file");
    hook::fail_step(Some(ProjectId::new(1)));
    let refused = library.import_file(&file);
    hook::fail_step(None);
    assert!(
        matches!(refused, Err(PortError::Corrupt { .. })),
        "{refused:?}"
    );
    let opened = library.import_file(&file).expect("the file opens");
    assert_eq!(artwork_of(&mut library, opened.get()).to_parts(), expected);
    drop(library);

    let now = records(&dir);
    for id in 2..=4 {
        assert_eq!(
            of_project(&now, id),
            of_project(&stored, id),
            "{id} as it was"
        );
    }
    let schema = now.iter().find(|(_, key, _)| key == SCHEMA_KEY);
    assert_eq!(schema.expect("schema").2, 22u32.to_le_bytes());
}
