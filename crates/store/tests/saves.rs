//! Save ids and the display cache (S54, DECISIONS D81): every save writes a new save id into the
//! head, two saves of the same content too; a head without one gets one when it is opened and
//! keeps it; a version rolled back to an older head holds that head's id, never the later
//! save's. The display cache's file gives back what was written, bit for bit, replaces it whole,
//! and refuses anything damaged.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use openpaint_app::{ProjectId, ProjectStore, SaveId, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_store::{CacheKey, DisplayCache, Library};

/// A directory that does not exist yet, under cargo's temporary directory for tests.
fn fresh(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "store-saves-{name}-{}-{unique}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear an old test directory");
    }
    dir
}

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

#[test]
fn every_save_writes_a_new_save_id_and_opening_gives_a_head_without_one_its_own() {
    let dir = fresh("ids");
    let mut library = Library::at(&dir);
    let artwork = blank();
    let id = library
        .create(&artwork, &Title::untitled())
        .expect("create");
    // A new project's head holds none until it is opened: opening writes one and keeps it.
    let opened = library.load(id).expect("load").saved;
    assert_eq!(library.load(id).expect("load again").saved, opened);
    drop(library);
    let mut library = Library::at(&dir);
    let last = library.open_last().expect("open").expect("the last");
    assert_eq!(last.saved, opened, "the id is in the head");
    // The same content saved twice: two new ids.
    let first = library.save(id, &artwork).expect("save");
    let second = library.save(id, &artwork).expect("save again");
    assert!(
        first != second && first != opened,
        "{first:?} {second:?} {opened:?}"
    );
    assert_eq!(library.load(id).expect("load").saved, second);
}

#[test]
fn a_library_rolled_back_holds_the_older_heads_id() {
    let dir = fresh("rollback");
    let mut library = Library::at(&dir);
    let id = library
        .create(&blank(), &Title::untitled())
        .expect("create");
    let older = library.save(id, &blank()).expect("save");
    drop(library);
    let backup = fresh("rollback-backup");
    copy_dir(&dir, &backup);
    let mut library = Library::at(&dir);
    let later = library.save(id, &blank()).expect("a later save");
    drop(library);
    std::fs::remove_dir_all(&dir).expect("the library goes");
    copy_dir(&backup, &dir);
    let mut library = Library::at(&dir);
    let restored = library.open_last().expect("open").expect("the last");
    assert_eq!(restored.saved, older);
    let again = library
        .save(id, &blank())
        .expect("a save after the restore");
    assert!(
        again != later && again != older,
        "a save never repeats an id"
    );
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("directory");
    for entry in std::fs::read_dir(from).expect("read") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("copy");
        }
    }
}

fn key(saved: u64) -> CacheKey {
    CacheKey {
        saved: SaveId::new(saved).expect("a save id"),
        renderer: 0x5eed,
        format: 10,
        size: [33, 7],
    }
}

/// Pixels a canvas of `size` might hold: paper with a few words of paint.
fn pixels([width, height]: [u32; 2], ink: u8) -> Vec<u8> {
    let mut pixels = vec![0xff; (width * height * 4) as usize];
    for (at, byte) in pixels.iter_mut().enumerate().step_by(37) {
        *byte = ink ^ (at as u8);
    }
    pixels
}

#[test]
fn a_cached_canvas_reads_back_bit_for_bit_and_is_replaced_whole() {
    let cache = DisplayCache::at(fresh("cache").join("canvas"));
    let project = ProjectId::new(7);
    assert!(
        cache.read(project).expect("read").is_none(),
        "none at first"
    );
    let first = pixels([33, 7], 3);
    let bytes = cache.write(project, &key(1), &first).expect("write");
    assert_eq!(
        bytes,
        std::fs::metadata(cache.path(project)).expect("file").len()
    );
    let read = cache.read(project).expect("read").expect("cached");
    assert_eq!((read.key, read.pixels), (key(1), first));
    let second = pixels([33, 7], 9);
    cache.write(project, &key(2), &second).expect("write again");
    let read = cache.read(project).expect("read").expect("cached");
    assert_eq!((read.key, read.pixels), (key(2), second));
    let dir = cache.path(project).parent().expect("dir").to_path_buf();
    let files: Vec<_> = std::fs::read_dir(&dir).expect("dir").collect();
    assert_eq!(files.len(), 1, "no temporary file stays");
    assert!(
        cache.write(project, &key(3), &[0; 12]).is_err(),
        "pixels that do not fill the size"
    );
    cache.delete(project).expect("delete");
    assert!(cache.read(project).expect("read").is_none(), "deleted");
    cache.delete(project).expect("deleting none succeeds");
}

#[test]
fn a_damaged_cache_file_is_refused() {
    let cache = DisplayCache::at(fresh("damaged"));
    let project = ProjectId::new(3);
    cache
        .write(project, &key(5), &pixels([33, 7], 1))
        .expect("write");
    let path = cache.path(project);
    let whole = std::fs::read(&path).expect("file");
    let last = whole.len() - 1;
    for broken in [
        whole[..last].to_vec(),
        [&whole[..last], &[whole[last] ^ 1]].concat(),
        [b"OPDISP02", &whole[8..]].concat(),
        Vec::new(),
    ] {
        std::fs::write(&path, &broken).expect("damage");
        assert!(cache.read(project).is_err(), "{} bytes", broken.len());
    }
}
