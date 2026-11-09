//! Record spaces (S110, D112): a space's records are bytes the store keeps and hands back as they
//! were, apart from every other space's and the store's own, listed whole or under a prefix; a
//! space or key the store's keys take is refused with nothing written; records of several spaces
//! written together, and a project created or removed with its records, land whole or not at all,
//! whichever write fails.

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

/// `bytes` under `key(space, id)`, as one entry of a write.
fn entry(space: Space, id: u8, bytes: &[u8]) -> (Space, Record) {
    (space, (key(space, id), bytes.to_vec()))
}

#[test]
fn a_space_s_records_come_back_as_they_were_apart_from_every_other() {
    let dir = TestDir::new("spaces");
    let mut library = Library::at(&dir);
    library.create(&sheet(), &Title::untitled()).expect("art");
    // Bytes no store record could decode: the store keeps them as they are.
    let odd: Vec<u8> = vec![0xff, 0, 0xfe, 7];
    library.put_records(&[entry(MINE, 2, &odd)]).expect("put");
    library.put_records(&[entry(MINE, 1, b"one")]).expect("put");
    library
        .put_records(&[entry(OTHER, 1, b"other")])
        .expect("put");
    let mine = library.records(MINE, &[]).expect("mine");
    assert_eq!(
        mine,
        [(key(MINE, 1), b"one".to_vec()), (key(MINE, 2), odd)],
        "in key order, its own alone"
    );
    library
        .put_records(&[entry(MINE, 1, b"again")])
        .expect("put");
    assert_eq!(library.records(MINE, &[]).expect("mine")[0].1, b"again");
    assert_eq!(library.records(OTHER, &[]).expect("other").len(), 1);
    drop(library);
    let mut reopened = Library::at(&dir);
    assert_eq!(reopened.records(MINE, &[]).expect("mine").len(), 2, "kept");
    assert_eq!(reopened.list().expect("list").len(), 1, "no project");
}

#[test]
fn a_space_s_records_under_a_prefix_are_those_alone() {
    let dir = TestDir::new("spaces-under");
    let mut library = Library::at(&dir);
    let under_one = (MINE, (vec![MINE.byte(), 1, 1], b"c".to_vec()));
    let written = [
        entry(MINE, 1, b"a"),
        entry(MINE, 2, b"b"),
        under_one.clone(),
        entry(OTHER, 1, b"o"),
    ];
    library.put_records(&written).expect("put");
    let under = |library: &mut Library, space, under: &[u8]| {
        let records = library.records(space, under).expect("records");
        records.into_iter().map(|(key, _)| key).collect::<Vec<_>>()
    };
    assert_eq!(
        under(&mut library, MINE, &[0]),
        [key(MINE, 1), key(MINE, 2)]
    );
    assert_eq!(under(&mut library, MINE, &[0, 2]), [key(MINE, 2)]);
    assert_eq!(under(&mut library, MINE, &[1]), [under_one.1.0]);
    assert_eq!(under(&mut library, MINE, &[]).len(), 3, "the whole space");
    assert!(under(&mut library, MINE, &[2]).is_empty());
    assert!(
        under(&mut library, OTHER, &[1]).is_empty(),
        "another space's"
    );
    assert_eq!(under(&mut library, OTHER, &[0]), [key(OTHER, 1)]);
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
        assert_eq!(library.records(taken, &[]), Err(refused.clone()), "{byte}");
        assert_eq!(library.records(taken, &[1]), Err(refused.clone()), "{byte}");
        let put = library.put_records(&[(taken, (vec![byte, 1], b"x".to_vec()))]);
        assert_eq!(put, Err(refused.clone()), "{byte}");
    }
    let outside = library.put_records(&[entry(MINE, 1, b"x"), (MINE, (key(OTHER, 1), vec![]))]);
    assert_eq!(outside, Err(refused.clone()), "one key outside its space");
    let created = library.create_owned((&sheet(), &Title::untitled()), &|_| {
        vec![entry(MINE, 1, b"x"), (MINE, (key(OTHER, 1), b"x".to_vec()))]
    });
    assert_eq!(created, Err(refused.clone()));
    let removed = library.remove_owned(&[(MINE, key(MINE, 1)), (MINE, b"last".to_vec())], None);
    assert_eq!(removed, Err(refused));
    drop(library);
    assert_eq!(dump(&dir), before, "nothing written");
}

/// The records a mode keeps for project `id`, one in each of two spaces: its id's bytes.
fn owned(id: ProjectId) -> Vec<(Space, Record)> {
    let bytes = id.get().to_le_bytes();
    vec![entry(MINE, 1, &bytes), entry(OTHER, 1, &bytes)]
}

/// A new library holding an artwork (project 1), open; with `owned`, also a project of records
/// in `MINE` and `OTHER` (project 2), the open one.
fn library_of(name: &str, with_owned: bool) -> (TestDir, Library) {
    let dir = TestDir::new(name);
    let mut library = Library::at(&dir);
    library.create(&sheet(), &Title::untitled()).expect("art");
    if with_owned {
        let made = library.create_owned((&sheet(), &Title::untitled()), &owned);
        assert_eq!(made, Ok(ProjectId::new(2)));
    }
    (dir, library)
}

/// How many writes `call` takes on a library `library_of` makes.
fn writes(with_owned: bool, call: impl Fn(&mut Library) -> Result<(), PortError>) -> usize {
    let (_dir, mut library) = library_of("spaces-count", with_owned);
    let start = faults::writes();
    call(&mut library).expect("whole");
    faults::writes() - start
}

/// `call` on a library `library_of` makes, its `n`-th write failing for every `n`: each time
/// the call fails and the library holds what it held before.
fn whole_or_nothing(
    name: &str,
    with_owned: bool,
    call: impl Fn(&mut Library) -> Result<(), PortError>,
) {
    let count = writes(with_owned, &call);
    assert!(count > 2, "{name}: more than one write and the commit");
    for n in 1..=count {
        let (dir, library) = library_of(name, with_owned);
        drop(library);
        let before = dump(&dir);
        let mut library = Library::at(&dir);
        library.list().expect("opened");
        faults::fail_write(n);
        assert!(call(&mut library).is_err(), "{name}: write {n} of {count}");
        drop(library);
        let after = dump(&dir);
        assert_eq!(
            after, before,
            "{name}: write {n} of {count} failed: nothing"
        );
    }
}

#[test]
fn records_of_two_spaces_are_written_whole_or_not_at_all() {
    let put = |library: &mut Library| {
        let records = [
            entry(MINE, 1, b"a"),
            entry(MINE, 2, b"b"),
            entry(OTHER, 1, b"c"),
        ];
        library.put_records(&records)
    };
    let (_dir, mut library) = library_of("spaces-put", false);
    put(&mut library).expect("written");
    assert_eq!(library.records(MINE, &[]).expect("mine").len(), 2);
    assert_eq!(library.records(OTHER, &[]).expect("other").len(), 1);
    whole_or_nothing("spaces-puts", false, put);
}

#[test]
fn a_project_with_its_records_is_created_whole_or_not_at_all() {
    let create = |library: &mut Library| {
        let made = library.create_owned((&sheet(), &Title::untitled()), &owned);
        made.map(drop)
    };
    let (_dir, mut library) = library_of("spaces-created", false);
    create(&mut library).expect("created");
    assert_eq!(
        library.open_last().expect("open").map(|p| p.id.get()),
        Some(2)
    );
    let [mine, other] = <[_; 2]>::try_from(owned(ProjectId::new(2))).expect("two");
    assert_eq!(library.records(MINE, &[]), Ok(vec![mine.1]));
    assert_eq!(library.records(OTHER, &[]), Ok(vec![other.1]));
    let listed = library.list().expect("list").len();
    assert_eq!(listed, 2, "listed as any project");
    whole_or_nothing("spaces-create", false, create);
}

#[test]
fn a_project_with_its_records_is_removed_whole_or_not_at_all() {
    let remove = |library: &mut Library| {
        let keys = [(MINE, key(MINE, 1)), (OTHER, key(OTHER, 1))];
        library.remove_owned(&keys, Some(ProjectId::new(2)))
    };
    let (_dir, mut library) = library_of("spaces-removed", true);
    remove(&mut library).expect("removed");
    assert_eq!(library.records(MINE, &[]), Ok(Vec::new()));
    assert_eq!(library.records(OTHER, &[]), Ok(Vec::new()));
    let listed: Vec<u64> = (library.list().expect("list").iter())
        .map(|item| item.id.get())
        .collect();
    assert_eq!(listed, [1], "the artwork stays");
    whole_or_nothing("spaces-remove", true, remove);
}
