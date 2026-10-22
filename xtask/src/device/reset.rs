//! `cargo device-reset` (S68): the app's data on the tablet cleared by Android (`pm clear`), every
//! file it held listed first. Refused unless a backup (`device-backup`) was written today (UTC,
//! as the backups' own times are), so what it clears is never the only copy; the backups stay.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::adb::{Adb, DeviceError, PACKAGE, Result};
use super::{artifacts, civil, stop_app};

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
    use std::time::Duration;

    use super::*;

    #[test]
    fn only_a_backup_written_today_counts() {
        let dir = crate::process::root()
            .join("target")
            .join("tmp")
            .join(format!("xtask-reset-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory");
        let today = day_of(SystemTime::now());
        assert!(matches!(todays_backup(&dir, today), Ok(None)), "empty");
        let note = dir.join("notes.txt");
        std::fs::write(&note, b"not a backup").expect("a file");
        assert!(matches!(todays_backup(&dir, today), Ok(None)), "no tar");
        let tar = dir.join("pre-1-0.tar");
        std::fs::write(&tar, b"a backup").expect("a file");
        assert_eq!(todays_backup(&dir, today).expect("read"), Some(tar.clone()));
        assert!(
            matches!(todays_backup(&dir, today + 1), Ok(None)),
            "yesterday's"
        );
        let missing = dir.join("missing");
        assert!(matches!(todays_backup(&missing, today), Ok(None)));
        let day = Duration::from_secs(86_400);
        assert_eq!(day_of(UNIX_EPOCH + day * 20_748), 20_748);
        std::fs::remove_file(&tar).expect("the test's tar goes");
        std::fs::remove_file(&note).expect("the test's note goes");
        std::fs::remove_dir(&dir).expect("the test's directory goes");
    }
}
