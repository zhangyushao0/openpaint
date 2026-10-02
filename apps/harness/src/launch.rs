//! Starting the app in the harness: each command gets a fresh library under `artifacts/`, so
//! no earlier run's artwork opens, and frames run until the library has opened the canvas.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use openpaint_runtime::Runtime;
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::output::workspace_root;

/// Longest wait for the library to open.
const OPEN_LIMIT: Duration = Duration::from_secs(10);

/// `dir` emptied: the library of an earlier run is removed.
pub fn fresh_library(dir: PathBuf) -> Result<PathBuf, HarnessError> {
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}

/// The app at `pixels_per_point` on a fresh library `artifacts/harness/<name>-library`, with
/// its canvas open on a screen of `size` pixels.
pub fn launch(name: &str, pixels_per_point: f32, size: [u32; 2]) -> Result<Runtime, HarnessError> {
    let dir = workspace_root()
        .join("artifacts")
        .join("harness")
        .join(format!("{name}-library"));
    let library = Library::at(fresh_library(dir)?);
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
