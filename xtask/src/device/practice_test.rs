//! `cargo device-input practice-test start [stage|copy]|end` (DECISIONS D102): a test practice
//! session (or a test colour copy) beside the author's, for a practice slice to walk on the
//! tablet, and removed afterwards with its sheet, so the author's sessions, copies, records and the
//! Today card's progress stay as they were.
//!
//! `start` goes to the gallery (a canvas showing is left by its Gallery button, nothing drawn),
//! starts the session the Today card would start next (of `stage` when given, as the practice
//! options' stage rows do; `copy`: a colour copy of a pack's picture), beside any open one, and
//! keeps its name
//! (id, sheet, seed, start) in `artifacts/device/practice-test`; it refuses while a name is kept.
//! `end` goes to the gallery, has the app remove the session of the name kept (its record and
//! sheet) and forgets the name; a name the app holds no session of (a library restored since)
//! is forgotten and nothing removed. The app matches all four parts, so no other session can go.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::adb::{Adb, DeviceError, Result, write_file};
use super::artifacts;
use super::input::{answer, instrument};
use super::state::{Screen, State, tap, until};

/// How long the app may take to get to the gallery.
const WAIT: Duration = Duration::from_secs(90);

/// The name kept between `start` and `end`.
fn kept() -> PathBuf {
    artifacts().join("practice-test")
}

fn failed(why: impl Into<String>) -> DeviceError {
    DeviceError::Failed {
        command: "device-input practice-test".into(),
        output: why.into(),
    }
}

/// `start [stage|copy]` or `end`.
pub(super) fn practice_test(adb: &Adb, what: &str, stage: &[&str]) -> Result<()> {
    let usage = DeviceError::Usage("cargo device-input practice-test start [stage|copy]|end");
    match (what, stage) {
        ("start", []) => start(adb, &kept(), "start"),
        ("start", ["copy"]) => start(adb, &kept(), "start:copy"),
        ("start", [stage]) => {
            let stage: u8 = stage.parse().map_err(|_| usage)?;
            start(adb, &kept(), &format!("start:{stage}"))
        }
        ("end", []) => end(adb, &kept()),
        _ => Err(usage),
    }
}

/// The gallery, listed: a canvas showing is left first.
fn to_gallery(adb: &Adb) -> Result<()> {
    if until(adb, WAIT, |s| s.screen != Screen::Starting)?.screen == Screen::Canvas {
        tap(adb, "Gallery")?;
    }
    until(adb, WAIT, State::listed).map(drop)
}

fn start(adb: &Adb, kept: &Path, command: &str) -> Result<()> {
    if let Ok(name) = std::fs::read_to_string(kept) {
        return Err(failed(format!(
            "test session {} is still kept in {}: end it first",
            name.trim(),
            kept.display()
        )));
    }
    to_gallery(adb)?;
    let output = instrument(adb, &[("practice-test", command.to_owned())])?;
    let said = answer(&output, "practice-test").unwrap_or("no answer");
    let name = said.strip_prefix("started ").filter(|name| named(name));
    let Some(name) = name else {
        return Err(failed(said));
    };
    write_file(kept, name.as_bytes())?;
    println!("started test session {name} (kept in {})", kept.display());
    println!("end it with: cargo device-input practice-test end");
    Ok(())
}

fn end(adb: &Adb, kept: &Path) -> Result<()> {
    let name = std::fs::read_to_string(kept)
        .map_err(|_| failed(format!("no test session kept in {}", kept.display())))?;
    let name = name.trim();
    if !named(name) {
        return Err(failed(format!(
            "{} holds no test session's name",
            kept.display()
        )));
    }
    to_gallery(adb)?;
    let output = instrument(adb, &[("practice-test", format!("end:{name}"))])?;
    let said = answer(&output, "practice-test").unwrap_or("no answer");
    let gone = said == format!("removed {name}") || said == "no such session";
    if !gone {
        return Err(failed(said));
    }
    std::fs::remove_file(kept).map_err(|error| DeviceError::File {
        path: kept.to_path_buf(),
        error: error.to_string(),
    })?;
    println!("{said}; forgot {}", kept.display());
    Ok(())
}

/// Whether `name` is a test session's name as the app writes it, `id:sheet:seed:started`: only
/// digits, small hex letters and three colons reach the device's shell.
fn named(name: &str) -> bool {
    let parts: Vec<&str> = name.split(':').collect();
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let hex = |part: &str| {
        !part.is_empty() && part.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    };
    matches!(parts[..], [id, sheet, seed, started] if digits(id) && digits(sheet) && hex(seed) && digits(started))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_test_sessions_name_reaches_the_device() {
        assert!(named("13:41:5eed004500000079:1791290000"));
        for bad in [
            "",
            "13:41:5eed",
            "13:41:5EED:1",
            "13:41:5eed:1;rm",
            "a:41:5:1",
            "1:2:3:4:5",
        ] {
            assert!(!named(bad), "{bad}");
        }
    }
}
