//! S50: a library that fails to write, a save cut off by a killed process, and an upgrade that
//! fails on one artwork, over the frozen samples (`tests/samples/`). Unit tests: they make writes
//! and steps fail through the test hooks in `batch.rs` and `steps.rs`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use fjall::{Database, KeyspaceCreateOptions, PersistMode};
use openpaint_app::{ProjectId, ProjectStore, Thumbnail, Title};
use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, Color, LayerCommand, StrokeSample,
    StrokeStyle,
};

use crate::Library;
use crate::batch::faults;
use crate::records::{LIBRARY, PROJECTS};

#[path = "robustness_upgrade_tests.rs"]
mod upgrade;

/// Records as the frozen samples hold them: keyspace, key, value.
type Dump = Vec<(&'static str, Vec<u8>, Vec<u8>)>;

/// The frozen sample of `schema`.
fn sample(schema: u32) -> Dump {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/samples/schema-{schema}.bin"));
    let bytes = std::fs::read(path).expect("the frozen sample");
    let mut rest = &bytes[..];
    let mut records = Vec::new();
    while let Some((&index, after)) = rest.split_first() {
        rest = after;
        let keyspace = [LIBRARY, PROJECTS][usize::from(index)];
        let mut part = || {
            let len = u32::from_le_bytes(take(&mut rest, 4).try_into().expect("a length"));
            take(&mut rest, len as usize).to_vec()
        };
        let (key, value) = (part(), part());
        records.push((keyspace, key, value));
    }
    records
}

/// The next `len` bytes of `rest`.
fn take<'a>(rest: &mut &'a [u8], len: usize) -> &'a [u8] {
    let (part, after) = rest.split_at(len);
    *rest = after;
    part
}

/// A directory that does not exist yet, under the target directory's `tmp` (as cargo's
/// `CARGO_TARGET_TMPDIR`, which unit tests are not given).
fn fresh(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp")
        .join(format!("store-unit-{name}-{}-{unique}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear an old test directory");
    }
    dir
}

/// A library in a new directory holding `records`, written past the store.
fn library_of(name: &str, records: &Dump) -> PathBuf {
    let dir = fresh(name);
    let db = Database::builder(&dir).open().expect("open raw");
    for (name, key, value) in records {
        let keyspace = db
            .keyspace(name, KeyspaceCreateOptions::default)
            .expect("keyspace");
        keyspace.insert(key.clone(), value.clone()).expect("insert");
    }
    db.persist(PersistMode::SyncAll).expect("persist");
    dir
}

/// Every record of the closed library in `dir`.
fn records(dir: &Path) -> Dump {
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

/// A copy of the closed library in `from`.
fn copied(from: &Path, name: &str) -> PathBuf {
    let to = fresh(name);
    copy_dir(from, &to);
    to
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("a directory") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("a copy");
        }
    }
}

/// `artwork` with a new layer holding one stroke of `samples` samples from `x`.
fn drawn(artwork: &Artwork, x: f32, samples: u16) -> Artwork {
    let layered = artwork
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .expect("a layer")
        .artwork;
    let recipe = Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 6.0, 1.0, Color::BLACK).expect("style");
    let sample = |i: u16| {
        let point = CanvasPoint::new(x + f32::from(i), 10.0).expect("point");
        StrokeSample::new(point, 0.5, 0.0, 0.0, u32::from(i) * 4).expect("sample")
    };
    let mut session = layered.begin_stroke(style, sample(0)).expect("begin");
    let rest: Vec<StrokeSample> = (1..samples).map(sample).collect();
    session.extend(&rest).expect("extend");
    session.finish().artwork
}

fn artwork_of(library: &mut Library, id: u64) -> Artwork {
    library.load(ProjectId::new(id)).expect("opens").artwork
}

fn thumbnail() -> Thumbnail {
    Thumbnail::new([1, 1], vec![1, 2, 3, 255]).expect("thumbnail")
}

/// The schema-22 sample (project 1) with a second artwork and a kept version of each, closed.
fn two_artworks() -> (PathBuf, Artwork, Artwork) {
    let dir = library_of("two", &sample(22));
    let mut library = Library::at(&dir);
    let one = artwork_of(&mut library, 1);
    let two = drawn(&one, 20.0, 6);
    let second = library.create(&two, &Title::untitled()).expect("create");
    assert_eq!(second, ProjectId::new(2));
    library
        .keep_version(ProjectId::new(1), 100)
        .expect("version");
    library.keep_version(second, 200).expect("version");
    library.load(ProjectId::new(1)).expect("the first is open");
    (dir, one, two)
}

/// What leaving project 1 as `artwork` does (the storage worker's `Leave`): the save, then the
/// thumbnail, a kept version and closing, each failure of these logged and passed; nothing
/// after a failed save. Returns whether the save landed.
fn leave(library: &mut Library, artwork: &Artwork) -> bool {
    let id = ProjectId::new(1);
    if library.save(id, artwork).is_err() {
        return false;
    }
    let _thumbnail = library.save_thumbnail(id, &thumbnail());
    let _version = library.keep_version(id, 300);
    let _closed = library.close();
    true
}

#[test]
fn a_write_failing_anywhere_in_leaving_keeps_the_last_complete_save_and_every_version() {
    let (base, one, two) = two_artworks();
    let later = drawn(&one, 30.0, 12);
    // How many writes leaving takes: every one of them fails in turn below.
    let counted = copied(&base, "count");
    let mut library = Library::at(&counted);
    artwork_of(&mut library, 1);
    let start = faults::writes();
    assert!(leave(&mut library, &later));
    let writes = faults::writes() - start;
    drop(library);
    let versions_before = versions(&base);
    assert!(
        writes > 8,
        "the save, thumbnail, version and close: {writes}"
    );
    let kept = versions(&counted).len() - versions_before.len();
    assert_eq!(kept, 2, "leaving kept a version: its file and its entry");
    for n in 1..=writes {
        let dir = copied(&base, "failing");
        let mut library = Library::at(&dir);
        artwork_of(&mut library, 1);
        faults::fail_write(n);
        let saved = leave(&mut library, &later);
        drop(library);
        let mut reopened = Library::at(&dir);
        let expected = if saved { &later } else { &one };
        let first = artwork_of(&mut reopened, 1).to_parts();
        assert_eq!(first, expected.to_parts(), "write {n} of {writes} failed");
        assert_eq!(artwork_of(&mut reopened, 2).to_parts(), two.to_parts());
        drop(reopened);
        // Every version kept before is there byte for byte; the new one whole or not at all.
        let now = versions(&dir);
        for version in &versions_before {
            assert!(now.contains(version), "write {n}: a version was lost");
        }
        let added: Vec<_> = now
            .iter()
            .filter(|v| !versions_before.contains(v))
            .collect();
        assert!(
            added.len() <= 2,
            "write {n}: its file and its entry, or neither"
        );
        assert!(added.len() != 1, "write {n}: half a version");
        std::fs::remove_dir_all(&dir).expect("the trial's copy goes");
    }
}

/// The kept versions' records (files in `projects`, entries in `library`).
fn versions(dir: &Path) -> Dump {
    let kept = |keyspace: &str, key: &[u8]| match keyspace {
        LIBRARY => key.first() == Some(&b'v'),
        _ => key.get(8) == Some(&b'v'),
    };
    records(dir)
        .into_iter()
        .filter(|(keyspace, key, _)| kept(keyspace, key))
        .collect()
}

/// Every journal file in `dir` and its length.
fn journals(dir: &Path) -> Vec<(PathBuf, u64)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("the library") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|ext| ext == "jnl") {
            let len = std::fs::metadata(&path).expect("a journal").len();
            found.push((path, len));
        }
    }
    found
}

#[test]
fn a_save_cut_off_anywhere_in_the_journal_reopens_at_the_last_complete_save() {
    let (base, _, _) = two_artworks();
    let before = journals(&base);
    let saved = copied(&base, "saved");
    let mut library = Library::at(&saved);
    let one = artwork_of(&mut library, 1);
    let later = drawn(&one, 30.0, 2);
    library.save(ProjectId::new(1), &later).expect("save");
    drop(library);
    // The journal the save went to, and where it ended before.
    let after = journals(&saved);
    let (journal, end) = after
        .iter()
        .filter(|(path, len)| {
            let name = path.file_name();
            let old = before.iter().find(|(p, _)| p.file_name() == name);
            old.is_none_or(|(_, old)| old < len)
        })
        .max_by_key(|(_, len)| *len)
        .cloned()
        .expect("the save's journal");
    let name = journal.file_name().expect("a name");
    let old = before.iter().find(|(p, _)| p.file_name() == Some(name));
    let start = old.map_or(0, |(_, len)| *len);
    assert!(
        end > start + 100,
        "the save is in the journal: {start}..{end}"
    );
    // Every byte, so every record boundary of the batch: four cuts at a time.
    let (one, later) = (one.to_parts(), later.to_parts());
    std::thread::scope(|scope| {
        for first in 0..4 {
            let (saved, one, later) = (&saved, &one, &later);
            scope.spawn(move || {
                for cut in (start + first..=end).step_by(4) {
                    let opened = cut_at(saved, name, cut);
                    let expected = if cut == end { later } else { one };
                    assert_eq!(opened, *expected, "cut at {cut} of {start}..{end}");
                }
            });
        }
    });
}

/// The library saved in `saved` with its journal `name` cut at `cut` bytes, as the process killed
/// there leaves it: the artwork it opens.
fn cut_at(saved: &Path, name: &std::ffi::OsStr, cut: u64) -> openpaint_core::ArtworkParts {
    let dir = copied(saved, "cut");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(dir.join(name))
        .expect("the journal");
    file.set_len(cut).expect("cut");
    drop(file);
    let mut library = Library::at(&dir);
    let opened = library.open_last().expect("opens").expect("a project");
    drop(library);
    std::fs::remove_dir_all(&dir).expect("the trial's copy goes");
    opened.artwork.to_parts()
}
