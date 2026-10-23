//! `TestDir`: a directory a runtime test makes for itself under cargo's temporary directory for
//! tests (`target/tmp`), removed when the test passes and kept, its path printed, when it fails
//! (S87). A test library is a fjall database whose journal takes 64 MiB: kept from every run,
//! they filled the disk.

use std::ffi::OsStr;
use std::io::{ErrorKind, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A new empty directory that its test owns: dropped while the test passes, it is removed with
/// everything in it; dropped while the test fails (its thread panicking), it is kept and named.
/// It owns exactly the directory it made, never one an earlier run left. Declare it before the
/// runtimes and libraries on it, so they close first (a test that restarts on it keeps it
/// through every start): Windows removes no file a database holds open. A removal that fails is
/// a printed warning, never a failure.
pub struct TestDir(PathBuf);

impl TestDir {
    /// `target/tmp/runtime-<name>-<process>-<n>`, made now; `n` counts past any name taken.
    pub fn new(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
        std::fs::create_dir_all(tmp).expect("the tests' temporary directory");
        loop {
            let unique = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = tmp.join(format!("runtime-{name}-{}-{unique}", std::process::id()));
            match std::fs::create_dir(&dir) {
                Ok(()) => return Self(dir),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => panic!("{}: {error}", dir.display()),
            }
        }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        // Straight to stderr: the test harness shows no captured line of a test that passes.
        let (mut stderr, shown) = (std::io::stderr(), self.0.display());
        if std::thread::panicking() {
            let _ = writeln!(stderr, "kept the failed test's directory {shown}");
        } else if let Err(error) = std::fs::remove_dir_all(&self.0)
            && error.kind() != ErrorKind::NotFound
        {
            let _ = writeln!(stderr, "warning: test directory {shown} stays: {error}");
        }
    }
}

/// The path, wherever a path is taken: `&dir` as `&Path`, `dir.join(..)`, `Library::at(&dir)`.
impl Deref for TestDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<OsStr> for TestDir {
    fn as_ref(&self) -> &OsStr {
        self.0.as_os_str()
    }
}
