//! The display cache through the whole frame loop (S54): an artwork opened without a cache fills
//! in and, once complete, its canvas is written to the cache under the save id its head holds; the
//! next start shows it complete from the frame it opens (nothing waits), equal to a cold
//! evaluation, and an edit after it is too. A cache written for another save (one more save of
//! the same content, or a library rolled back from a backup to an older head), another renderer,
//! another format or another size is never taken as complete: the canvas fills as before and then
//! writes the cache anew. A test binary of its own: it makes its device's frames' share small, so
//! a fill takes many frames.

mod gpu;
mod hand;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use hand::{Hand, assert_cold};
use openpaint_app::{ProjectId, ProjectStore, SaveId, Title};
use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
};
use openpaint_runtime::{DisplayCache, Runtime, ToolKind};
use openpaint_store::{CacheKey, Library};

const SIZE: [u32; 2] = [1200, 900];
const PIXELS_PER_POINT: f32 = 2.0;
const CANVAS: [u32; 2] = [480, 320];
const PROJECT: ProjectId = ProjectId::new(1);

/// A library directory and a cache directory, neither there yet.
struct Dirs {
    library: PathBuf,
    cache: PathBuf,
}

fn fresh(name: &str) -> Dirs {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("display-{name}-{}-{unique}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("clear an old test directory");
    }
    Dirs {
        library: root.join("library"),
        cache: root.join("cache"),
    }
}

/// `count` strokes across the canvas at many angles, `shift` moving them all.
fn strokes(count: u32, shift: f32) -> Artwork {
    let mut artwork = Artwork::new(CanvasSize::new(CANVAS[0], CANVAS[1]).expect("size"));
    for k in 0..count {
        let angle = k as f32 * 2.399_963_2;
        let samples: Vec<StrokeSample> = (0..12u16)
            .map(|i| {
                let s = 200.0 * (f32::from(i) / 11.0 - 0.5);
                let at = CanvasPoint::new(240.0 + shift + s * angle.cos(), 160.0 + s * angle.sin());
                StrokeSample::new(at.expect("point"), 0.7, 0.0, 0.0, u32::from(i) * 8)
                    .expect("sample")
            })
            .collect();
        let colour = Color::rgb(0.1 + 0.1 * (k % 7) as f32, 0.4, 0.7).expect("colour");
        let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 24.0, 0.6, colour);
        let mut stroke = artwork
            .begin_stroke(style.expect("style"), samples[0])
            .expect("begin");
        stroke.extend(&samples[1..]).expect("extend");
        artwork = stroke.finish().artwork;
    }
    artwork
}

/// A runtime on the library and cache of `dirs`, run until its canvas opened; whether tiles
/// waited in the frame it opened.
fn open(dirs: &Dirs) -> (Runtime, bool) {
    let core = gpu::core();
    // Each frame takes one piece: a fill takes many frames.
    core.set_frame_budget(10_000.0);
    let stores = (
        Box::new(Library::at(&dirs.library)) as Box<dyn ProjectStore>,
        DisplayCache::at(&dirs.cache),
    );
    let mut runtime =
        Runtime::start_on_cached(core, PIXELS_PER_POINT, stores).expect("the runtime starts");
    let mut frames = 0;
    while runtime.artwork().is_none() {
        runtime.frame_offscreen(SIZE).expect("frame");
        frames += 1;
        assert!(frames < 10_000, "the artwork never opened");
    }
    let filling = runtime.canvas_filling();
    (runtime, filling)
}

/// Frames until nothing fills and every worker is done (the cache written).
fn settle(runtime: &mut Runtime) {
    let mut frames = 0;
    while runtime.frame_offscreen(SIZE).expect("frame") || runtime.moving() {
        frames += 1;
        assert!(frames < 100_000, "the runtime never rested");
    }
}

/// The cache of the project as the directory holds it now.
fn cached(dirs: &Dirs) -> CacheKey {
    let cache = DisplayCache::at(&dirs.cache).read(PROJECT).expect("read");
    cache.expect("a cached canvas").key
}

/// The save id the project's head holds.
fn head(dirs: &Dirs) -> SaveId {
    Library::at(&dirs.library)
        .load(PROJECT)
        .expect("load")
        .saved
}

/// The cache rewritten with `change` made to its key (and pixels of its size).
fn alter(dirs: &Dirs, change: impl FnOnce(&mut CacheKey)) {
    let cache = DisplayCache::at(&dirs.cache);
    let mut held = cache.read(PROJECT).expect("read").expect("cached");
    change(&mut held.key);
    let [width, height] = held.key.size;
    held.pixels.resize((width * height * 4) as usize, 0xff);
    cache
        .write(PROJECT, &held.key, &held.pixels)
        .expect("write");
}

/// Opens `dirs` and checks the canvas is not taken as complete: it fills, equals a cold
/// evaluation, and its cache is written anew under the head's save id.
fn fills_and_writes_anew(dirs: &Dirs, when: &str) {
    let (mut runtime, filling) = open(dirs);
    assert!(filling, "{when}: the cache was taken as complete");
    settle(&mut runtime);
    assert_cold(&gpu::core(), &mut runtime, when);
    drop(runtime);
    assert_eq!(cached(dirs).saved, head(dirs), "{when}: written anew");
}

#[test]
fn a_canvas_opens_complete_from_its_cache_equal_to_cold_and_stays_so_after_an_edit() {
    let dirs = fresh("complete");
    let artwork = strokes(40, 0.0);
    let mut library = Library::at(&dirs.library);
    library
        .create(&artwork, &Title::untitled())
        .expect("create");
    drop(library);
    let (mut runtime, filling) = open(&dirs);
    assert!(filling, "no cache yet: the canvas fills");
    settle(&mut runtime);
    assert_cold(&gpu::core(), &mut runtime, "filled");
    drop(runtime);
    let key = cached(&dirs);
    assert_eq!((key.saved, key.size), (head(&dirs), CANVAS));
    assert_eq!(key.renderer, gpu::core().renderer_identity());

    let (mut runtime, filling) = open(&dirs);
    assert!(!filling, "complete from the frame it opened");
    assert_cold(&gpu::core(), &mut runtime, "shown from the cache");
    let middle = [SIZE[0] as f32 / 2.0, SIZE[1] as f32 / 2.0];
    let line: Vec<[f32; 2]> = (0..12)
        .map(|i| {
            [
                middle[0] - 120.0 + 20.0 * i as f32,
                middle[1] + 7.0 * i as f32,
            ]
        })
        .collect();
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line);
    settle(&mut runtime);
    assert_cold(
        &gpu::core(),
        &mut runtime,
        "a stroke over the cached canvas",
    );
    let drawn = runtime.artwork().expect("artwork").clone();
    to_the_gallery_and_back(&mut runtime, &mut hand);
    drop(runtime);
    assert_ne!(
        cached(&dirs).saved,
        key.saved,
        "the stroke's save rewrote it"
    );
    assert_eq!(cached(&dirs).saved, head(&dirs));
    let (mut runtime, filling) = open(&dirs);
    assert!(!filling, "complete again");
    assert_eq!(
        runtime.artwork().expect("artwork").to_parts(),
        drawn.to_parts()
    );
    assert_cold(&gpu::core(), &mut runtime, "shown from the rewritten cache");
}

/// To the gallery and back by the artwork's card: the gallery's opening reads the cache too,
/// and the canvas is complete from the frame it opens.
fn to_the_gallery_and_back(runtime: &mut Runtime, hand: &mut Hand) {
    hand.tap(runtime, "Gallery");
    settle(runtime);
    assert!(runtime.artwork().is_none(), "in the gallery");
    hand.tap(runtime, ("Open artwork", [0.5, 0.5]));
    let mut frames = 0;
    while runtime.artwork().is_none() {
        runtime.frame_offscreen(SIZE).expect("frame");
        frames += 1;
        assert!(frames < 10_000, "the card never opened");
    }
    assert!(!runtime.canvas_filling(), "complete from its card");
    assert_cold(&gpu::core(), runtime, "opened from its card");
}

#[test]
fn a_cache_of_another_save_renderer_format_or_size_is_never_complete() {
    let dirs = fresh("stale");
    let artwork = strokes(30, 10.0);
    let mut library = Library::at(&dirs.library);
    library
        .create(&artwork, &Title::untitled())
        .expect("create");
    drop(library);
    fills_and_writes_anew(&dirs, "no cache");
    // One more save of the very same content: a new save id.
    Library::at(&dirs.library)
        .save(PROJECT, &artwork)
        .expect("save");
    fills_and_writes_anew(&dirs, "another save");
    alter(&dirs, |key| key.renderer ^= 1);
    fills_and_writes_anew(&dirs, "another renderer");
    alter(&dirs, |key| key.format = 18 - key.format);
    fills_and_writes_anew(&dirs, "another format");
    alter(&dirs, |key| key.size[1] -= 1);
    fills_and_writes_anew(&dirs, "another size");
    let (_, filling) = open(&dirs);
    assert!(!filling, "complete once written anew");
}

#[test]
fn a_library_rolled_back_never_takes_a_later_saves_cache_as_complete() {
    let dirs = fresh("rollback");
    let older = strokes(30, -20.0);
    let mut library = Library::at(&dirs.library);
    library.create(&older, &Title::untitled()).expect("create");
    drop(library);
    // The backup: the library as it is now.
    let backup = dirs.library.with_file_name("backup");
    copy_dir(&dirs.library, &backup);
    // A later save, whose canvas the cache then holds.
    Library::at(&dirs.library)
        .save(PROJECT, &strokes(31, -20.0))
        .expect("a later save");
    fills_and_writes_anew(&dirs, "the later save");
    let later = cached(&dirs);
    std::fs::remove_dir_all(&dirs.library).expect("the library goes");
    copy_dir(&backup, &dirs.library);
    let (mut runtime, filling) = open(&dirs);
    assert!(filling, "the later save's cache is not taken as complete");
    settle(&mut runtime);
    assert_eq!(
        runtime.artwork().expect("artwork").to_parts(),
        older.to_parts()
    );
    assert_cold(&gpu::core(), &mut runtime, "rolled back");
    drop(runtime);
    assert_ne!(
        cached(&dirs).saved,
        later.saved,
        "written anew for the older head"
    );
}

fn copy_dir(from: &Path, to: &Path) {
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
