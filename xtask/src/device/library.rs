//! `cargo device-backup [name]` and `cargo device-restore <name|file.tar>`: the app's library
//! (`files/library`) streamed out of the app's private storage into
//! `artifacts/device/backups/<name>.tar`, listed and checked; and put back: the app stopped (no
//! process left), the library as found saved first as `<name>-found-<time>.tar` (listed), then
//! the archive written back in place (nothing is deleted), read out again and compared with the
//! backup file by file, byte for byte.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, PACKAGE, Result, read_file, write_file};
use super::{artifacts, plain, tar, timestamp};
use crate::process::root;

/// The library inside the app's private storage, relative to its data directory.
const LIBRARY: &str = "files/library";
/// How long `stop_app` waits for the app's process to end after `am force-stop`, and a restore
/// for the device's `tar` to end.
const STOP_WAIT: Duration = Duration::from_secs(5);
/// How long a restore waits before it reads the library back a second time.
const SETTLE_WAIT: Duration = Duration::from_secs(2);

pub fn backup(args: &[String]) -> Result<()> {
    let name = match args {
        [] => format!("library-{}", timestamp()),
        [name] => plain(name)?.to_owned(),
        _ => return Err(DeviceError::Usage("cargo device-backup [name]")),
    };
    save_found(&Adb::connected()?, &name)
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
    stop_app(&adb)?;
    // Whatever the library holds now (strokes made during a test window) is kept first.
    let stem = path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    save_found(&adb, &format!("{stem}-found-{}", timestamp()))?;
    adb.feed(&["exec-in", "run-as", PACKAGE, "tar", "xf", "-"], &archive)?;
    // `exec-in` returns once adb has sent the archive, not once the device's tar has written
    // it: reading back at once can catch the last member (the journal) half written.
    wait_for_extraction(&adb)?;
    let mut differences = tar::differences(&backup, &tar::entries(&pull_library(&adb)?)?);
    if !differences.is_empty() {
        println!(
            "read back differs ({}); verifying again after {} s",
            differences.join("; "),
            SETTLE_WAIT.as_secs()
        );
        std::thread::sleep(SETTLE_WAIT);
        differences = tar::differences(&backup, &tar::entries(&pull_library(&adb)?)?);
    }
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

/// The library as the device's `tar` writes it.
fn pull_library(adb: &Adb) -> Result<Vec<u8>> {
    adb.bytes(&["exec-out", "run-as", PACKAGE, "tar", "cf", "-", LIBRARY])
}

/// The library as the device holds it now into `artifacts/device/backups/<name>.tar`, listed;
/// an archive of that name already there is never replaced.
fn save_found(adb: &Adb, name: &str) -> Result<()> {
    let path = artifacts().join("backups").join(format!("{name}.tar"));
    if path.exists() {
        return Err(DeviceError::Exists(path));
    }
    let archive = pull_library(adb)?;
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

/// Force-stops the app and waits until no process of it is left (`pidof` prints nothing), so
/// nothing writes the library afterwards; a process still there after `STOP_WAIT` fails.
pub(super) fn stop_app(adb: &Adb) -> Result<()> {
    adb.shell(&["am", "force-stop", PACKAGE])?;
    let started = Instant::now();
    loop {
        // `pidof` exits 1 when nothing runs; `; true` keeps that from being a failure.
        let pids = adb.shell(&[&format!("pidof {PACKAGE}; true")])?;
        let pids = pids.trim();
        if pids.is_empty() {
            return Ok(());
        }
        if started.elapsed() >= STOP_WAIT {
            return Err(DeviceError::StillRunning(pids.to_owned()));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Waits (at most `STOP_WAIT`) until no `tar` runs on the device, so the extraction `exec-in`
/// started has written every member; still running then, the read-back's retry covers it.
fn wait_for_extraction(adb: &Adb) -> Result<()> {
    let started = Instant::now();
    while started.elapsed() < STOP_WAIT {
        if adb.shell(&["pidof tar; true"])?.trim().is_empty() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!(
        "a tar still runs on the device after {} s",
        STOP_WAIT.as_secs()
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
