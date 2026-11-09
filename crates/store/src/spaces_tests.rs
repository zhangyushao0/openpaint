//! Record spaces (S110, D112): a space's records are bytes the store keeps and hands back as they
//! were, apart from every other space's and the store's own; a space or key the store's keys take
//! is refused with nothing written; a project created or removed with its record lands whole or
//! not at all, whichever write fails.

use std::path::Path;

use fjall::{Database, KeyspaceCreateOptions};
use openpaint_app::{PortError, ProjectId, ProjectStore, Record, Space, Title};
use openpaint_core::{Artwork, CanvasSize};

use crate::Library;
use crate::batch::faults;
use crate::records::{LIBRARY, PROJECTS};
use crate::test_dir::TestDir;

const MINE: Space = Space::new(b'm');
const OTHER: Space = Space::new(b'o');

/// Every record of the closed library in `dir`: keyspace, key, value.
fn dump(dir: &Path) -> Vec<(&'static str, Vec<u8>, Vec<u8>)> {
    let db = Database::builder(dir).open().expect("open raw");
    let mut out = Vec::new();
    for name in [LIBRARY, PROJECTS] {
        let keyspace = db
            .keyspace(name, KeyspaceCreateOptions::default)
            .expect("keyspace");
        for guard in keyspace.iter() {
            let (key, value) = guard.into_inner().expect("record");
            out.push((name, key.to_vec(), value.to_vec()));
        }
    }
    out
}

fn sheet() -> Artwork {
    Artwork::new(CanvasSize::new(40, 30).expect("size"))
}

fn key(space: Space, id: u8) -> Vec<u8> {
    vec![space.byte(), 0, id]
}

#[test]
fn a_space_s_records_come_back_as_they_were_apart_from_every_other() {
    let dir = TestDir::new("spaces");
    let mut library = Library::at(&dir);
    library.create(&sheet(), &Title::untitled()).expect("art");
    // Bytes no store record could decode: the store keeps them as they are.
    let odd: Vec<u8> = vec![0xff, 0, 0xfe, 7];
    library.put_record(MINE, &key(MINE, 2), &odd).expect("put");
    library
        .put_record(MINE, &key(MINE, 1), b"one")
        .expect("put");
    library
        .put_record(OTHER, &key(OTHER, 1), b"other")
        .expect("put");
    let mine = library.records(MINE).expect("mine");
    assert_eq!(
        mine,
        [(key(MINE, 1), b"one".to_vec()), (key(MINE, 2), odd)],
        "in key order, its own alone"
    );
    library
        .put_record(MINE, &key(MINE, 1), b"again")
        .expect("put");
    assert_eq!(library.records(MINE).expect("mine")[0].1, b"again");
    assert_eq!(library.records(OTHER).expect("other").len(), 1);
    drop(library);
    let mut reopened = Library::at(&dir);
    assert_eq!(reopened.records(MINE).expect("mine").len(), 2, "kept");
    assert_eq!(reopened.list().expect("list").len(), 1, "no project");
}

#[test]
fn a_space_or_key_the_store_s_own_keys_take_is_refused_and_nothing_written() {
    let dir = TestDir::new("spaces-refused");
    let mut library = Library::at(&dir);
    library.create(&sheet(), &Title::untitled()).expect("art");
    drop(library);
    let before = dump(&dir);
    let refused = PortError::Corrupt {
        what: "record space",
    };
    let mut library = Library::at(&dir);
    for byte in *b"acefnlv" {
        let taken = Space::new(byte);
        assert_eq!(library.records(taken), Err(refused.clone()), "{byte}");
        let put = library.put_record(taken, &[byte, 1], b"x");
        assert_eq!(put, Err(refused.clone()), "{byte}");
    }
    let outside = library.put_record(MINE, &key(OTHER, 1), b"x");
    assert_eq!(outside, Err(refused.clone()), "a key outside its space");
    let created = library.create_owned((&sheet(), &Title::untitled()), MINE, &|_| {
        (key(OTHER, 1), b"x".to_vec())
    });
    assert_eq!(created, Err(refused.clone()));
    let removed = library.remove_owned(MINE, b"last", None);
    assert_eq!(removed, Err(refused));
    drop(library);
    assert_eq!(dump(&dir), before, "nothing written");
}

/// The record a mode keeps for project `id`: its id's bytes.
fn record(id: ProjectId) -> Record {
    (key(MINE, 1), id.get().to_le_bytes().to_vec())
}

/// A new library holding an artwork (project 1), open; with `owned`, also a project of `MINE`'s
/// record (project 2), the open one.
fn library_of(name: &str, owned: bool) -> (TestDir, Library) {
    let dir = TestDir::new(name);
    let mut library = Library::at(&dir);
    library.create(&sheet(), &Title::untitled()).expect("art");
    if owned {
        let made = library.create_owned((&sheet(), &Title::untitled()), MINE, &record);
        assert_eq!(made, Ok(ProjectId::new(2)));
    }
    (dir, library)
}

/// How many writes `call` takes on a library `library_of` makes.
fn writes(owned: bool, call: impl Fn(&mut Library) -> Result<(), PortError>) -> usize {
    let (_dir, mut library) = library_of("spaces-count", owned);
    let start = faults::writes();
    call(&mut library).expect("whole");
    faults::writes() - start
}

#[test]
fn a_project_with_its_record_is_created_whole_or_not_at_all() {
    let create = |library: &mut Library| {
        let made = library.create_owned((&sheet(), &Title::untitled()), MINE, &record);
        made.map(drop)
    };
    let (dir, mut library) = library_of("spaces-created", false);
    create(&mut library).expect("created");
    assert_eq!(
        library.open_last().expect("open").map(|p| p.id.get()),
        Some(2)
    );
    assert_eq!(library.records(MINE), Ok(vec![record(ProjectId::new(2))]));
    assert_eq!(
        library.list().expect("list").len(),
        2,
        "listed as any project"
    );
    drop((library, dir));
    let count = writes(false, create);
    assert!(
        count > 4,
        "the project, its head, entry, counters and record"
    );
    for n in 1..=count {
        let (dir, mut library) = library_of("spaces-create", false);
        drop(library);
        let before = dump(&dir);
        library = Library::at(&dir);
        library.list().expect("opened");
        faults::fail_write(n);
        assert!(create(&mut library).is_err(), "write {n} of {count}");
        drop(library);
        assert_eq!(dump(&dir), before, "write {n} of {count} failed: nothing");
    }
}

#[test]
fn a_project_with_its_record_is_removed_whole_or_not_at_all() {
    let remove =
        |library: &mut Library| library.remove_owned(MINE, &key(MINE, 1), Some(ProjectId::new(2)));
    let (dir, mut library) = library_of("spaces-removed", true);
    remove(&mut library).expect("removed");
    assert_eq!(library.records(MINE), Ok(Vec::new()));
    let listed: Vec<u64> = (library.list().expect("list").iter())
        .map(|item| item.id.get())
        .collect();
    assert_eq!(listed, [1], "the artwork stays");
    drop((library, dir));
    let count = writes(true, remove);
    for n in 1..=count {
        let (dir, library) = library_of("spaces-remove", true);
        drop(library);
        let before = dump(&dir);
        let mut library = Library::at(&dir);
        library.list().expect("opened");
        faults::fail_write(n);
        assert!(remove(&mut library).is_err(), "write {n} of {count}");
        drop(library);
        assert_eq!(dump(&dir), before, "write {n} of {count} failed: nothing");
    }
}
