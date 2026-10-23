//! `cargo device-backup [name]` and `cargo device-restore <name|file.tar>`: the app's library
//! (`files/library`) and the practice packs kept beside it (`files/packs`, when there are any)
//! streamed out of the app's private storage into `artifacts/device/backups/<name>.tar`, listed
//! and checked; and put back: the app stopped (no process left), the library as found saved
//! first as `<name>-found-<time>.tar` (listed), then the archive written back in place (nothing
//! of the library is deleted; a pack the app kept after the backup is removed by its exact path),
//! read out again and compared with the backup file by file, byte for byte.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, PACKAGE, Result, read_file, write_file};
use super::{artifacts, plain, tar, timestamp};
use crate::process::root;

/// The library inside the app's private storage, relative to its data directory, and the
/// practice packs kept beside it (S80), when there are any.
const LIBRARY: &str = "files/library";
const PACKS: &str = "files/packs";
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
    let inside = |path: &str| path.starts_with(LIBRARY) || path.starts_with(PACKS);
    if let Some(stray) = backup.iter().find(|entry| !inside(&entry.path)) {
        return Err(DeviceError::Tar(format!(
            "{} is outside {LIBRARY} and {PACKS}",
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
    remove_packs_since(&adb, &backup)?;
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

/// The library, and the practice packs when the app keeps any, as the device's `tar` writes
/// them.
fn pull_library(adb: &Adb) -> Result<Vec<u8>> {
    let listing = adb.shell(&["run-as", PACKAGE, "ls", "files"])?;
    let packs = listing.split_whitespace().any(|name| name == "packs");
    let mut command = vec!["exec-out", "run-as", PACKAGE, "tar", "cf", "-", LIBRARY];
    if packs {
        command.push(PACKS);
    }
    adb.bytes(&command)
}

/// The practice packs the app kept after `backup` was taken (an import during a test window),
/// removed one by one by their exact paths (the found archive saved them first), and their
/// directory when the backup had none: a pack is whole files the app reads, so the packs are as
/// backed up.
fn remove_packs_since(adb: &Adb, backup: &[tar::Entry]) -> Result<()> {
    let found = tar::entries(&pull_library(adb)?)?;
    let held = |path: &str| backup.iter().any(|entry| entry.path == path);
    for entry in found.iter().filter(|e| e.kind == b'0' && !held(&e.path)) {
        let Some(name) = entry.path.strip_prefix(&format!("{PACKS}/")) else {
            continue;
        };
        if pack_file(name) {
            adb.shell(&["run-as", PACKAGE, "rm", &entry.path])?;
            println!("  removed {} (kept after the backup)", entry.path);
        }
    }
    let had_packs = backup.iter().any(|entry| entry.path.starts_with(PACKS));
    let has_packs = found.iter().any(|entry| entry.path.starts_with(PACKS));
    if has_packs && !had_packs {
        adb.shell(&["run-as", PACKAGE, "rmdir", PACKS])?;
        println!("  removed {PACKS}/ (none at the backup)");
    }
    Ok(())
}

/// Whether `name` is a pack file the app writes: a-z, 0-9 and '-', then `.pack` or `.pack.part`.
fn pack_file(name: &str) -> bool {
    let stem = name
        .strip_suffix(".pack")
        .or_else(|| name.strip_suffix(".pack.part"));
    stem.is_some_and(|stem| {
        !stem.is_empty()
            && stem
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
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

#[cfg(test)]
mod tests {
    use super::pack_file;

    #[test]
    fn only_pack_files_are_ever_removed() {
        for name in [
            "openpaint-shapes.pack",
            "personal.pack",
            "personal.pack.part",
        ] {
            assert!(pack_file(name), "{name}");
        }
        for name in [
            "",
            ".pack",
            "../library.pack",
            "a b.pack",
            "x.png",
            "OPEN.pack",
            "x.pack/",
        ] {
            assert!(!pack_file(name), "{name}");
        }
    }
}
