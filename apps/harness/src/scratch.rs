//! Directories the harness makes for itself, gone when the work that made them is done (S87):
//! `Scratch` under `target/tmp` for a test (removed when it passes, kept and named when it
//! fails) or a command's passing step (the test artwork's library), and `remove`, which the
//! scenario runner calls on a passing scenario's library. A library is a fjall database whose
//! journal takes 64 MiB: kept from every test run, they filled the disk.

use std::ffi::OsStr;
use std::io::{self, ErrorKind, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::output::workspace_root;

/// A new empty directory that its owner holds: dropped, it is removed with everything in it;
/// dropped while its thread panics (a failing test), it is kept and named. It owns exactly the
/// directory it made, never one an earlier run left. Declare it before the libraries in it, so
/// they close first: Windows removes no file a database holds open.
pub struct Scratch(PathBuf);

impl Scratch {
    /// `target/tmp/<name>-<process>-<n>`, made now; `n` counts past any name taken.
    ///
    /// # Errors
    /// The file system's, when the directory cannot be made.
    pub fn new(name: &str) -> io::Result<Self> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let tmp = workspace_root().join("target").join("tmp");
        std::fs::create_dir_all(&tmp)?;
        loop {
            let unique = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = tmp.join(format!("{name}-{}-{unique}", std::process::id()));
            match std::fs::create_dir(&dir) {
                Ok(()) => return Ok(Self(dir)),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // Straight to stderr: the test harness shows no captured line of a test that passes.
            let shown = self.0.display();
            let _ = writeln!(io::stderr(), "kept the failed test's directory {shown}");
        } else {
            remove(&self.0);
        }
    }
}

/// `dir` removed with everything in it, if it is there. A removal that fails is a printed
/// warning, never a failure.
pub fn remove(dir: &Path) {
    if let Err(error) = std::fs::remove_dir_all(dir)
        && error.kind() != ErrorKind::NotFound
    {
        let shown = dir.display();
        let _ = writeln!(io::stderr(), "warning: {shown} stays: {error}");
    }
}

/// The path, wherever a path is taken: `&dir` as `&Path`, `dir.join(..)`, `Library::at(&dir)`.
impl Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<OsStr> for Scratch {
    fn as_ref(&self) -> &OsStr {
        self.0.as_os_str()
    }
}
