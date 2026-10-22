//! Device chores on the tablet over adb, one command each, so no slice hand-rolls them:
//!
//! - `cargo device-backup [name]` and `cargo device-restore <name|file.tar>` (`library.rs`): the
//!   app's library out of the app's private storage into `artifacts/device/backups/`, and back in
//!   place, compared byte for byte;
//! - `cargo device-check [label] [--no-install]` (`check.rs`) and `cargo device-input ...`
//!   (`input.rs`);
//! - `cargo device-clean`: removes the agent's own test files, and only those (the test folders
//!   and screen recordings);
//! - `cargo device-push <file>`: a PNG or JPEG for the system picker, or an OpenPaint file for
//!   `device-input open-file`, into `/sdcard/Download/openpaint-test` (a test location `clean`
//!   removes; `push.rs`);
//! - `cargo device-info [label]`: the display's colour facts, read-only (`info.rs`); with
//!   `--watch <seconds>` the clocks, the GPU's load, the battery's draw and the app's CPU time
//!   over that time (`watch.rs`);
//! - `cargo device-smoke`: the installed debug build walked through every main feature on a
//!   test artwork it creates and deletes, about 20 s (`smoke.rs`);
//! - `cargo device-reset`: the app's data cleared by `pm clear`, each file it held listed;
//!   refused without a backup written today (`reset.rs`).
//!
//! Every adb call takes explicit arguments, never a host shell, so no path is rewritten.

mod adb;
mod check;
mod info;
mod input;
mod library;
mod open_file;
mod push;
mod reset;
mod smoke;
mod smoke_checks;
mod state;
mod tar;
mod watch;

use std::path::PathBuf;

use crate::process::root;
use adb::{Adb, Result};

pub use adb::DeviceError;
pub use check::check;
pub use info::info;
pub use input::input;
pub use library::{backup, restore};
pub use push::push;
pub use reset::reset;
pub use smoke::smoke;

/// The agent's test locations: never anything else (`clean`).
pub(crate) const TEST_DIRS: [&str; 2] = [
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

pub fn clean(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(DeviceError::Usage("cargo device-clean"));
    }
    clean_on(&Adb::connected()?).map(drop)
}

/// The agent's test files away, each listed; how many test locations went.
fn clean_on(adb: &Adb) -> Result<usize> {
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
        return Ok(0);
    }
    adb.shell(&[&format!("rm -rf {targets}")])?;
    for path in &found {
        println!("  removed {path}");
    }
    println!("clean: {} test locations removed", found.len());
    Ok(found.len())
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
