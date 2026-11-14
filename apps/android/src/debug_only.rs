//! What only the debug build carries stays out of a release: the library is one for every APK
//! (`jni_debug.rs`), so the debug source set alone may reach its debug entry points. The recorder
//! of frames in real use is made only after `DebugApplication` (debug source set) switched it on.

use std::path::{Path, PathBuf};

/// The Kotlin host's source sets.
pub(crate) fn source_set(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("app")
        .join("src")
        .join(name)
}

/// Every file under `dir` with its text.
pub(crate) fn texts(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(texts(&path));
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            found.push((path, text));
        }
    }
    found
}

#[test]
fn the_recorder_of_frames_is_switched_on_by_the_debug_build_alone() {
    let main = texts(&source_set("main"));
    assert!(
        main.iter()
            .any(|(path, _)| path.ends_with("MainActivity.kt"))
    );
    for (path, text) in &main {
        for debug_only in ["DebugControls", "recordFrames", "DebugApplication"] {
            assert!(
                !text.contains(debug_only),
                "{} names {debug_only}: a release would carry it",
                path.display()
            );
        }
    }
    let debug = texts(&source_set("debug"));
    let has = |file: &str, text: &str| {
        debug
            .iter()
            .any(|(path, held)| path.ends_with(file) && held.contains(text))
    };
    assert!(has(
        "AndroidManifest.xml",
        "android:name=\".DebugApplication\""
    ));
    assert!(has("DebugApplication.kt", "DebugControls.recordFrames()"));
}
