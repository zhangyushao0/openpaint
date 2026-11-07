//! The tests' directories: `Scratch` under `tools/packs/target/tmp`, removed when the test
//! passes, kept and named when it fails (as the harness keeps its own, S87).

use std::ffi::OsStr;
use std::io::{self, ErrorKind, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A new empty directory its owner holds: dropped, it is removed with everything in it; dropped
/// while its thread panics (a failing test), it is kept and named.
pub struct Scratch(PathBuf);

impl Scratch {
    /// `tools/packs/target/tmp/<name>-<process>-<n>`, made now; `n` counts past any name taken.
    ///
    /// # Errors
    /// The file system's, when the directory cannot be made.
    pub fn new(name: &str) -> io::Result<Self> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let tmp = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp");
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
        let shown = self.0.display();
        if std::thread::panicking() {
            let _ = writeln!(io::stderr(), "kept the failed test's directory {shown}");
        } else if let Err(error) = std::fs::remove_dir_all(&self.0)
            && error.kind() != ErrorKind::NotFound
        {
            let _ = writeln!(io::stderr(), "warning: {shown} stays: {error}");
        }
    }
}

/// The path, wherever a path is taken: `&dir` as `&Path`, `dir.join(..)`.
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

/// `Fetcher::cache_only(&dir)`: a path from it.
impl AsRef<OsStr> for Scratch {
    fn as_ref(&self) -> &OsStr {
        self.0.as_os_str()
    }
}
