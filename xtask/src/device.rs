//! Device chores on the tablet over adb, one command each, so no slice hand-rolls them:
//!
//! - `cargo device-backup [name]`: the app's library (`files/library`) streamed out of the app's
//!   private storage into `artifacts/device/backups/<name>.tar`, listed and checked;
//! - `cargo device-restore <name|file.tar>`: that archive written back in place (nothing is
//!   deleted), read out again and compared with the backup file by file, byte for byte;
//! - `cargo device-check [label] [--no-install]` and `cargo device-input ...` (`check.rs`);
//! - `cargo device-clean`: removes the agent's own test files, and only those.
//!
//! Every adb call takes explicit arguments, never a host shell, so no path is rewritten.

mod adb;
mod check;
mod tar;

use std::path::{Path, PathBuf};

use crate::process::root;
use adb::{Adb, PACKAGE, Result, read_file, write_file};

pub use adb::DeviceError;
pub use check::{check, input};

/// The library inside the app's private storage, relative to its data directory.
const LIBRARY: &str = "files/library";
/// The agent's test locations: never anything else (`clean`).
const TEST_DIRS: [&str; 2] = [
    "/sdcard/Pictures/openpaint-test",
    "/sdcard/Download/openpaint-test",
];
/// The agent's screen recordings, `/sdcard/openpaint-*.mp4`, quoted for the device's shell.
const TEST_RECORDINGS: &str = "'/sdcard/openpaint-'*'.mp4'";

/// `artifacts/device/`.
fn artifacts() -> PathBuf {
    root().join("artifacts").join("device")
}

/// Names become file names: a-z, 0-9 and '-'.
fn plain(name: &str) -> Result<&str> {
    let ok = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    ok.then_some(name)
        .ok_or(DeviceError::Usage("names use a-z, 0-9 and '-' only"))
}

/// The library as the device's `tar` writes it.
fn pull_library(adb: &Adb) -> Result<Vec<u8>> {
    adb.bytes(&["exec-out", "run-as", PACKAGE, "tar", "cf", "-", LIBRARY])
}

pub fn backup(args: &[String]) -> Result<()> {
    let name = match args {
        [] => format!("library-{}", timestamp()),
        [name] => plain(name)?.to_owned(),
        _ => return Err(DeviceError::Usage("cargo device-backup [name]")),
    };
    let path = artifacts().join("backups").join(format!("{name}.tar"));
    if path.exists() {
        return Err(DeviceError::Exists(path));
    }
    let adb = Adb::connected()?;
    let archive = pull_library(&adb)?;
    let entries = tar::entries(&archive)?;
    write_file(&path, &archive)?;
    for entry in &entries {
        println!("  {} {}", entry.path, entry.bytes.len());
    }
    let files = entries.iter().filter(|entry| entry.kind == b'0').count();
    println!(
        "backup {}: {} entries, {files} files, {} bytes",
        path.display(),
        entries.len(),
        archive.len()
    );
    Ok(())
}

pub fn restore(args: &[String]) -> Result<()> {
    let [which] = args else {
        return Err(DeviceError::Usage("cargo device-restore <name|file.tar>"));
    };
    let path = backup_path(which)?;
    let archive = read_file(&path)?;
    let backup = tar::entries(&archive)?;
    if let Some(stray) = backup.iter().find(|entry| !entry.path.starts_with(LIBRARY)) {
        return Err(DeviceError::Tar(format!(
            "{} is outside {LIBRARY}",
            stray.path
        )));
    }
    let adb = Adb::connected()?;
    // The app must not write while its library is put back.
    adb.shell(&["am", "force-stop", PACKAGE])?;
    adb.feed(&["exec-in", "run-as", PACKAGE, "tar", "xf", "-"], &archive)?;
    let now = tar::entries(&pull_library(&adb)?)?;
    let differences = tar::differences(&backup, &now);
    if !differences.is_empty() {
        return Err(DeviceError::Differs(differences));
    }
    println!(
        "restored {}: {} entries, every one byte-identical to the backup",
        path.display(),
        backup.len()
    );
    Ok(())
}

/// A backup's name (`artifacts/device/backups/<name>.tar`) or an archive's path.
fn backup_path(which: &str) -> Result<PathBuf> {
    if which.ends_with(".tar") {
        let path = Path::new(which);
        return Ok(if path.exists() {
            path.to_path_buf()
        } else {
            root().join(path)
        });
    }
    Ok(artifacts()
        .join("backups")
        .join(format!("{}.tar", plain(which)?)))
}

pub fn clean(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(DeviceError::Usage("cargo device-clean"));
    }
    let adb = Adb::connected()?;
    let quoted: Vec<String> = TEST_DIRS.iter().map(|dir| format!("'{dir}'")).collect();
    let targets = format!("{} {TEST_RECORDINGS}", quoted.join(" "));
    // `ls` names what exists; a missing test location is no error.
    let found = adb.shell(&[&format!("ls -d {targets} 2>/dev/null; true")])?;
    let found: Vec<&str> = found
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if found.is_empty() {
        println!("clean: no test files on the device");
        return Ok(());
    }
    adb.shell(&[&format!("rm -rf {targets}")])?;
    for path in &found {
        println!("  removed {path}");
    }
    println!("clean: {} test locations removed", found.len());
    Ok(())
}

/// The time now (UTC) as `YYYYMMDD-HHMMSS`.
fn timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let (days, day) = (seconds / 86_400, seconds % 86_400);
    let (year, month, date) = civil(days);
    format!(
        "{year:04}{month:02}{date:02}-{:02}{:02}{:02}",
        day / 3600,
        day / 60 % 60,
        day % 60
    )
}

/// Year, month and day of `days` since 1970-01-01 (Howard Hinnant's civil-from-days).
fn civil(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let date = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_names() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_729), (2026, 10, 3));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert!(plain("s22-before").is_ok());
        assert!(plain("../x").is_err() && plain("").is_err() && plain("A").is_err());
    }

    #[test]
    fn clean_reaches_only_the_test_locations() {
        for dir in TEST_DIRS {
            assert!(dir.starts_with("/sdcard/") && dir.ends_with("/openpaint-test"));
        }
        assert_eq!(TEST_RECORDINGS, "'/sdcard/openpaint-'*'.mp4'");
    }
}
