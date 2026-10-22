//! Starting the app in the harness: each command gets a fresh library under `artifacts/`, so
//! no earlier run's artwork opens (or a fresh copy of the library it reads), and frames run until
//! the library has opened the canvas.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use openpaint_runtime::{FontFile, Runtime, TextFonts};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::output::workspace_root;

/// Longest wait for the library to open.
const OPEN_LIMIT: Duration = Duration::from_secs(10);

/// The development machine's stand-ins for the faces the Android host reads from the system,
/// first found wins. Latin: Roboto where installed, else Segoe UI (Windows' plain sans); Chinese:
/// Noto Sans SC (the same design as the tablet's Noto Sans CJK, held at regular weight), else
/// Microsoft YaHei.
const LATIN_FONTS: [&str; 2] = [
    "C:/Windows/Fonts/Roboto-Regular.ttf",
    "C:/Windows/Fonts/segoeui.ttf",
];
const CJK_FONTS: [&str; 2] = [
    "C:/Windows/Fonts/NotoSansSC-VF.ttf",
    "C:/Windows/Fonts/msyh.ttc",
];

/// The text faces as a host would hand them over; a face not found is left out (Latin falls
/// back to the built-in face, Chinese shows as boxes).
pub fn text_fonts() -> TextFonts {
    let first = |paths: &[&str]| {
        paths.iter().find_map(|path| {
            let bytes = fs::read(path).ok()?;
            Some(FontFile { bytes, index: 0 })
        })
    };
    let fonts = TextFonts {
        latin: first(&LATIN_FONTS),
        cjk: first(&CJK_FONTS),
    };
    if fonts.latin.is_none() || fonts.cjk.is_none() {
        println!("a system text face is missing; the built-in face stands in");
    }
    fonts
}

/// `dir` emptied: the library of an earlier run is removed.
pub fn fresh_library(dir: PathBuf) -> Result<PathBuf, HarnessError> {
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}

/// `artifacts/harness/<name>-library`, the library of the command `name`.
pub fn library_dir(name: &str) -> PathBuf {
    workspace_root()
        .join("artifacts")
        .join("harness")
        .join(format!("{name}-library"))
}

/// A fresh copy of the library in `dir` as the command `name`'s library: opening a library
/// writes to it (a head gains a save id, the last artwork is marked open), so a tool reading one
/// (the author's, a backup's) opens a copy, never the library it was pointed at.
pub fn library_copy(dir: &str, name: &str) -> Result<PathBuf, HarnessError> {
    let from = Path::new(dir);
    if !from.is_dir() {
        return Err(HarnessError::Missing(
            "the command needs a library directory",
        ));
    }
    let copy = fresh_library(library_dir(name))?;
    copy_dir(from, &copy)?;
    Ok(copy)
}

/// Every file under `from` copied to the same place under `to`.
fn copy_dir(from: &Path, to: &Path) -> Result<(), HarnessError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// The app at `pixels_per_point` on a fresh library `artifacts/harness/<name>-library`, with
/// its canvas open on a screen of `size` pixels.
pub fn launch(name: &str, pixels_per_point: f32, size: [u32; 2]) -> Result<Runtime, HarnessError> {
    let library = Library::at(fresh_library(library_dir(name))?);
    let mut runtime = Runtime::start(pixels_per_point, Box::new(library))?;
    open(&mut runtime, size)?;
    Ok(runtime)
}

/// Frames of `size` until the canvas is open.
fn open(runtime: &mut Runtime, size: [u32; 2]) -> Result<(), HarnessError> {
    let start = Instant::now();
    while runtime.artwork().is_none() {
        if start.elapsed() > OPEN_LIMIT {
            return Err(HarnessError::Missing("the library did not open"));
        }
        runtime.frame_offscreen(size)?;
    }
    Ok(())
}
