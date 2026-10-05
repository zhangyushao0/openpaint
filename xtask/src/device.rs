//! Device chores on the tablet over adb, one command each, so no slice hand-rolls them:
//!
//! - `cargo device-backup [name]`: the app's library (`files/library`) streamed out of the app's
//!   private storage into `artifacts/device/backups/<name>.tar`, listed and checked;
//! - `cargo device-restore <name|file.tar>`: the app stopped (no process left), the library as found
//!   saved first as `<name>-found-<time>.tar` (listed), then that archive written back in place
//!   (nothing is deleted), read out again and compared with the backup file by file, byte for byte;
//! - `cargo device-check [label] [--no-install]` and `cargo device-input ...` (`check.rs`);
//! - `cargo device-clean`: removes the agent's own test files, and only those (the test folders,
//!   recordings, and test libraries an investigation left in the app's cache);
//! - `cargo device-push <picture>`: a PNG or JPEG into `/sdcard/Download/openpaint-test` (a test
//!   location `clean` removes) for the system picker;
//! - `cargo device-info [label]`: the display's colour facts, read-only (`info.rs`).
//!
//! Every adb call takes explicit arguments, never a host shell, so no path is rewritten.

mod adb;
mod check;
mod info;
mod tar;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::process::root;
use adb::{Adb, PACKAGE, Result, read_file, write_file};

pub use adb::DeviceError;
pub use check::{check, input};
pub use info::info;

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
    save_found(&Adb::connected()?, &name)
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

/// How long `stop_app` waits for the app's process to end after `am force-stop`.
const STOP_WAIT: Duration = Duration::from_secs(5);

/// Force-stops the app and waits until no process of it is left (`pidof` prints nothing), so
/// nothing writes the library afterwards; a process still there after `STOP_WAIT` fails.
fn stop_app(adb: &Adb) -> Result<()> {
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

/// How long a restore waits before it reads the library back a second time.
const SETTLE_WAIT: Duration = Duration::from_secs(2);

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
    if !found.is_empty() {
        adb.shell(&[&format!("rm -rf {targets}")])?;
    }
    for path in &found {
        println!("  removed {path}");
    }
    let cached = clean_cache(&adb)?;
    if found.is_empty() && cached == 0 {
        println!("clean: no test files on the device");
        return Ok(());
    }
    println!("clean: {} test locations removed", found.len() + cached);
    Ok(())
}

/// Test libraries an investigation left in the app's cache: `cache/inv1-library` and names
/// that continue it with letters, digits, '-' or '_', each listed, then removed by its name.
const TEST_CACHE: &str = "cache/inv1-library";

fn clean_cache(adb: &Adb) -> Result<usize> {
    // adb joins the words into one line for the device's shell: the inner command is quoted.
    let listed = adb.shell(&[&format!(
        "run-as {PACKAGE} sh -c 'ls -d {TEST_CACHE}* 2>/dev/null; true'"
    )])?;
    let names: Vec<&str> = listed
        .lines()
        .map(str::trim)
        .filter(|name| test_cache(name))
        .collect();
    for name in &names {
        adb.shell(&["run-as", PACKAGE, "rm", "-rf", name])?;
        println!("  removed {PACKAGE}/{name}");
    }
    Ok(names.len())
}

/// Whether `name` is one of the investigation's cached test libraries.
fn test_cache(name: &str) -> bool {
    name.strip_prefix(TEST_CACHE).is_some_and(|rest| {
        rest.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

/// Where `push` puts a picture: one of the test locations `clean` removes.
const PUSHED: &str = "/sdcard/Download/openpaint-test";

/// The file name a pushed picture keeps: a-z, 0-9 and '-', then `.png`, `.jpg` or `.jpeg`.
fn picture_name(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    let picture = matches!(extension, "png" | "jpg" | "jpeg");
    if picture && plain(stem).is_ok() {
        Ok(name.to_owned())
    } else {
        Err(DeviceError::Usage(
            "cargo device-push <picture.png|.jpg>, named with a-z, 0-9 and '-'",
        ))
    }
}

/// `cargo device-push <file>`: a picture into `PUSHED` for the system picker, made known to
/// the device's media index so the picker lists it.
pub fn push(args: &[String]) -> Result<()> {
    let [file] = args else {
        return Err(DeviceError::Usage("cargo device-push <picture.png|.jpg>"));
    };
    let local = Path::new(file);
    let local = if local.exists() {
        local.to_path_buf()
    } else {
        root().join(local)
    };
    let name = picture_name(&local)?;
    let bytes = read_file(&local)?.len();
    let adb = Adb::connected()?;
    adb.shell(&["mkdir", "-p", PUSHED])?;
    let target = format!("{PUSHED}/{name}");
    let local_text = local.to_string_lossy().into_owned();
    adb.text(&["push", &local_text, &target])?;
    let uri = format!("file://{target}");
    adb.shell(&[
        "am",
        "broadcast",
        "-a",
        "android.intent.action.MEDIA_SCANNER_SCAN_FILE",
        "-d",
        &uri,
    ])?;
    println!("pushed {} ({bytes} bytes) to {target}", local.display());
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
        let name = |path: &str| picture_name(Path::new(path)).ok();
        assert_eq!(
            name("tests/fixtures/s26-photo.jpg").as_deref(),
            Some("s26-photo.jpg")
        );
        assert!(name("a b.png").is_none() && name("x.txt").is_none() && name("png").is_none());
        assert!(PUSHED.starts_with("/sdcard/") && TEST_DIRS.contains(&PUSHED));
    }

    #[test]
    fn clean_reaches_only_the_test_locations() {
        for dir in TEST_DIRS {
            assert!(dir.starts_with("/sdcard/") && dir.ends_with("/openpaint-test"));
        }
        assert_eq!(TEST_RECORDINGS, "'/sdcard/openpaint-'*'.mp4'");
        assert!(test_cache("cache/inv1-library") && test_cache("cache/inv1-library-2"));
        for other in [
            "cache/inv1",
            "files/library",
            "cache/inv1-library/../x",
            "cache/inv1-library x",
        ] {
            assert!(!test_cache(other), "{other}");
        }
    }
}
