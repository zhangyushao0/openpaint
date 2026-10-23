//! `TestDir`: a directory a host test makes for itself under `target/tmp` (S87), removed with
//! everything in it when the test passes, kept and its path printed when it fails.

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A new empty directory that its test owns, never one an earlier run left.
pub struct TestDir(PathBuf);

impl TestDir {
    /// `target/tmp/android-<name>-<process>-<n>`, made now; `n` counts past any name taken.
    pub fn new(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = crate_dir.ancestors().nth(2).expect("the workspace");
        let tmp = workspace.join("target").join("tmp");
        std::fs::create_dir_all(&tmp).expect("the tests' temporary directory");
        loop {
            let unique = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = tmp.join(format!("android-{name}-{}-{unique}", std::process::id()));
            match std::fs::create_dir(&dir) {
                Ok(()) => return Self(dir),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => panic!("{}: {error}", dir.display()),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        // Straight to stderr: the test harness shows no captured line of a test that passes.
        let (mut stderr, shown) = (std::io::stderr(), self.0.display());
        if std::thread::panicking() {
            let _ = writeln!(stderr, "kept the failed test's directory {shown}");
        } else if let Err(error) = std::fs::remove_dir_all(&self.0) {
            let _ = writeln!(stderr, "warning: test directory {shown} stays: {error}");
        }
    }
}
