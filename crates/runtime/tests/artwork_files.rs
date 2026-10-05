//! GPU test of OpenPaint files and kept versions through the whole frame loop on a real
//! library (DECISIONS D62, D63): the store's schema-14 sample artwork (strokes of every brush,
//! fills, smudges, masks, groups, clipping, adjustments, blurs, transforms and a placed picture)
//! saved as a file from its card's options and opened again under "+" shows the same canvas
//! and holds the same artwork; leaving it unchanged keeps no version, leaving it changed keeps
//! one, and that version restores as a copy showing what was left.

mod gpu;
mod hand;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use hand::{Hand, assert_cold};
use openpaint_app::{GalleryNotice, GalleryProjection, ProjectStore, ScreenProjection};
use openpaint_runtime::{HostAnswer, HostRequest, ImageSink, ImageSource, PortError, Runtime};
use openpaint_store::Library;

const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

fn library_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("runtime-files-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear an old library");
    }
    dir
}

/// The store's frozen schema-14 sample as an OpenPaint file titled "Hafen": its project's
/// records, the file layout written out by hand.
fn sample_file() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../store/tests/samples/schema-14.bin"
    );
    let dump = std::fs::read(path).expect("the store's sample");
    let (mut at, mut records) = (0, Vec::new());
    let take = |len: usize, at: &mut usize| {
        let part = dump[*at..*at + len].to_vec();
        *at += len;
        part
    };
    while at < dump.len() {
        let keyspace = take(1, &mut at)[0];
        let part = |at: &mut usize| {
            let len = u32::from_le_bytes(take(4, at).try_into().expect("length"));
            take(len as usize, at)
        };
        let (key, value) = (part(&mut at), part(&mut at));
        if keyspace == 1 && key[..8] == 1u64.to_be_bytes() {
            records.push((key[8..].to_vec(), value));
        }
    }
    let mut file = b"OpenPaint\0".to_vec();
    let blob = |file: &mut Vec<u8>, part: &[u8]| {
        file.extend_from_slice(&u32::try_from(part.len()).expect("len").to_le_bytes());
        file.extend_from_slice(part);
    };
    file.extend_from_slice(&14u32.to_le_bytes());
    blob(&mut file, b"Hafen");
    file.extend_from_slice(&u32::try_from(records.len()).expect("count").to_le_bytes());
    for (key, value) in &records {
        blob(&mut file, key);
        blob(&mut file, value);
    }
    let check = crc32fast::hash(&file);
    file.extend_from_slice(&check.to_le_bytes());
    file
}

struct Saved(Arc<Mutex<Vec<u8>>>);

impl ImageSink for Saved {
    fn write(self: Box<Self>, bytes: &[u8]) -> Result<(), PortError> {
        *self.0.lock().expect("saved") = bytes.to_vec();
        Ok(())
    }
}

struct Chosen(Vec<u8>);

impl ImageSource for Chosen {
    fn read(self: Box<Self>, _limit: u64) -> Result<Vec<u8>, PortError> {
        Ok(self.0)
    }
}

/// Frames until `ready` holds; panics after 20 seconds.
fn until(runtime: &mut Runtime, ready: impl Fn(&Runtime) -> bool) {
    let start = Instant::now();
    while runtime.frame_offscreen(SIZE).expect("frame") || !ready(runtime) {
        assert!(start.elapsed().as_secs() < 20, "never got there");
    }
}

fn gallery(runtime: &Runtime) -> Option<GalleryProjection> {
    match runtime.screen() {
        ScreenProjection::Gallery(gallery) if gallery.listed() => Some(gallery),
        _ => None,
    }
}

fn items(runtime: &Runtime) -> usize {
    gallery(runtime).map_or(0, |gallery| gallery.items().len())
}

/// The only card saved as a file through its options and the system's save dialog, then that
/// file opened under "+" through the system's picker.
fn save_and_open(runtime: &mut Runtime, hand: &mut Hand) {
    hand.tap(runtime, "Artwork options");
    hand.tap(runtime, "Save as OpenPaint file…");
    let name = "Hafen.openpaint".to_owned();
    assert_eq!(
        runtime.host_request(),
        Some(HostRequest::SaveArtwork { name })
    );
    let saved = Arc::new(Mutex::new(Vec::new()));
    runtime.answer(HostAnswer::File(Box::new(Saved(Arc::clone(&saved)))));
    until(runtime, |runtime| {
        gallery(runtime).is_some_and(|g| g.notice() == GalleryNotice::FileSaved)
    });
    hand.tap(runtime, "OK");
    hand.tap(runtime, "New artwork");
    hand.tap(runtime, "Open OpenPaint file…");
    assert_eq!(runtime.host_request(), Some(HostRequest::OpenArtwork));
    let file = saved.lock().expect("saved").clone();
    runtime.answer(HostAnswer::Image(Box::new(Chosen(file))));
    until(runtime, |runtime| items(runtime) == 2);
}

/// The first card's only version restored from its options; returns the version's date as
/// the row showed it.
fn restore_the_version(runtime: &mut Runtime, hand: &mut Hand) -> String {
    let before = items(runtime);
    hand.tap(runtime, "Artwork options #1");
    let names = runtime.controls().names();
    let version = names
        .iter()
        .find(|name| name.starts_with(|c: char| c.is_ascii_digit()) && name.contains(':'))
        .expect("the version's date on screen")
        .clone();
    hand.tap(runtime, version.as_str());
    until(runtime, |runtime| items(runtime) == before + 1);
    version
}

#[test]
fn a_saved_file_opens_as_the_same_artwork_and_a_version_restores_what_was_left() {
    let core = gpu::core();
    let dir = library_dir("round-trip");
    Library::at(&dir)
        .import_file(&sample_file())
        .expect("the sample");
    let store = Box::new(Library::at(&dir));
    let mut runtime =
        Runtime::start_on(Arc::clone(&core), PIXELS_PER_POINT, store).expect("the runtime starts");
    let mut hand = Hand::new(SIZE);
    until(&mut runtime, |runtime| items(runtime) == 1);
    hand.tap(&mut runtime, "Open artwork");
    until(&mut runtime, |runtime| runtime.artwork().is_some());
    assert_cold(&core, &runtime, "the sample opened");
    let original = runtime.artwork().expect("artwork").to_parts();
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert!(runtime.back(), "to the gallery");
    until(&mut runtime, |runtime| items(runtime) == 1);
    let listed = gallery(&runtime).expect("gallery");
    assert!(
        listed.items()[0].versions.is_empty(),
        "left unchanged: no version"
    );

    save_and_open(&mut runtime, &mut hand);
    // The new card comes first.
    hand.tap(&mut runtime, "Open artwork #1");
    until(&mut runtime, |runtime| runtime.artwork().is_some());
    assert_eq!(runtime.artwork().expect("artwork").to_parts(), original);
    let opened = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(
        opened.content_hash(),
        canvas.content_hash(),
        "the same canvas"
    );

    // A change: a new layer above the active one.
    hand.tap(&mut runtime, "Layers");
    hand.tap(&mut runtime, "Add layer");
    hand.tap(&mut runtime, "Layers");
    let left = runtime.artwork().expect("artwork").to_parts();
    assert_ne!(left, original);
    let drawn = runtime.read_canvas().expect("readback").expect("canvas");
    assert!(runtime.back(), "to the gallery");
    until(&mut runtime, |runtime| {
        gallery(runtime).is_some_and(|g| g.items()[0].versions.len() == 1)
    });
    let version = restore_the_version(&mut runtime, &mut hand);
    let listed = gallery(&runtime).expect("gallery");
    assert_eq!(
        listed.items()[0].title.as_str(),
        format!("Hafen ({version})")
    );
    hand.tap(&mut runtime, "Open artwork #1");
    until(&mut runtime, |runtime| runtime.artwork().is_some());
    let restored = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(
        restored.content_hash(),
        drawn.content_hash(),
        "what was left"
    );
    assert_eq!(runtime.artwork().expect("artwork").to_parts(), left);
}
