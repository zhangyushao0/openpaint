//! `cargo device-reset`: the app's data on the tablet cleared by Android (`pm clear`), every
//! file it held listed first. Refused unless a backup (`device-backup`) was written today (UTC,
//! as the backups' own times are), so what it clears is never the only copy; the backups stay.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::adb::{Adb, DeviceError, PACKAGE, Result};
use super::library::stop_app;
use super::{artifacts, civil};

pub fn reset(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(DeviceError::Usage("cargo device-reset"));
    }
    let backups = artifacts().join("backups");
    let today = day_of(SystemTime::now());
    let backup = todays_backup(&backups, today)?.ok_or(DeviceError::NoBackup(backups))?;
    println!("today's backup: {}", backup.display());
    let adb = Adb::connected()?;
    let held = app_files(&adb)?;
    stop_app(&adb)?;
    let answer = adb.shell(&["pm", "clear", PACKAGE])?;
    if answer.trim() != "Success" {
        return Err(DeviceError::Failed {
            command: format!("pm clear {PACKAGE}"),
            output: answer,
        });
    }
    for file in &held {
        println!("  cleared {file}");
    }
    let left = app_files(&adb)?;
    for file in &left {
        println!("  left {file}");
    }
    println!(
        "reset {PACKAGE}: {} files cleared, {} left",
        held.len(),
        left.len()
    );
    Ok(())
}

/// Every file in the app's data directory, relative to it, with its size.
fn app_files(adb: &Adb) -> Result<Vec<String>> {
    let listed = adb.shell(&[&format!(
        "run-as {PACKAGE} sh -c 'find . -type f -exec ls -s {{}} + 2>/dev/null; true'"
    )])?;
    Ok(listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

/// The UTC day of `time`: days since 1970-01-01.
fn day_of(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / 86_400)
}

/// A backup (`<dir>/*.tar`) written on `day`, if any.
fn todays_backup(dir: &Path, day: u64) -> Result<Option<PathBuf>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    for entry in entries {
        let entry = entry.map_err(|error| DeviceError::File {
            path: dir.to_path_buf(),
            error: error.to_string(),
        })?;
        let path = entry.path();
        let written = entry.metadata().and_then(|meta| meta.modified());
        let tar = path.extension().is_some_and(|ext| ext == "tar");
        if tar && written.is_ok_and(|written| day_of(written) == day) {
            let (year, month, date) = civil(day);
            println!("a backup of {year:04}-{month:02}-{date:02} (UTC) is kept");
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::io::{ErrorKind, Write};
    use std::time::Duration;

    use super::*;

    #[test]
    fn only_a_backup_written_today_counts() {
        let held = TestDir::new("xtask-reset");
        let dir = held.0.as_path();
        let today = day_of(SystemTime::now());
        assert!(matches!(todays_backup(dir, today), Ok(None)), "empty");
        let note = dir.join("notes.txt");
        std::fs::write(&note, b"not a backup").expect("a file");
        assert!(matches!(todays_backup(dir, today), Ok(None)), "no tar");
        let tar = dir.join("pre-1-0.tar");
        std::fs::write(&tar, b"a backup").expect("a file");
        assert_eq!(todays_backup(dir, today).expect("read"), Some(tar.clone()));
        assert!(
            matches!(todays_backup(dir, today + 1), Ok(None)),
            "yesterday's"
        );
        let missing = dir.join("missing");
        assert!(matches!(todays_backup(&missing, today), Ok(None)));
        let day = Duration::from_secs(86_400);
        assert_eq!(day_of(UNIX_EPOCH + day * 20_748), 20_748);
    }

    /// The test's own new directory under `target/tmp` (S87): removed with everything in it when
    /// the test passes, kept and named when it fails; never one an earlier run left.
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let tmp = crate::process::root().join("target").join("tmp");
            std::fs::create_dir_all(&tmp).expect("the tests' temporary directory");
            let mut n = 0;
            loop {
                let dir = tmp.join(format!("{name}-{}-{n}", std::process::id()));
                match std::fs::create_dir(&dir) {
                    Ok(()) => return Self(dir),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => n += 1,
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
            } else if let Err(error) = std::fs::remove_dir_all(&self.0) {
                let _ = writeln!(stderr, "warning: test directory {shown} stays: {error}");
            }
        }
    }
}
