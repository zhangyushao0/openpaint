//! `cargo device-input practice-test start [stage [n]|copy|drill <kind>]|end` (DECISIONS D102): a
//! test practice session (or a test colour copy or drill) beside the author's, for a practice
//! slice to walk on the tablet, and removed afterwards with its sheet, so the author's sessions,
//! drills, copies, records and the Today card's progress stay as they were.
//!
//! `start` goes to the gallery (a canvas showing is left by its Gallery button, nothing drawn),
//! starts the session the Today card would start next (of `stage` when given, as the practice
//! options' stage rows do; as the stage's `n`-th would be when that is given, so a stage's every
//! other session is reached; `copy`: a colour copy of a pack's picture; `drill <kind>`: a drill of
//! the kind its tile starts, named in a-z and '-' as `line`, `centre` or `next-face`, the app
//! refusing another name with the list), beside any open one, and keeps its name
//! (id, sheet, start) in `artifacts/device/practice-test`; it refuses while a name is kept.
//! `end` goes to the gallery, has the app remove the session of the name kept (its record and
//! sheet) and forgets the name; a name the app holds no session of (a library restored since)
//! is forgotten and nothing removed. The app matches all three parts, so no other session can go.

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

/// `start [stage [n]|copy|drill <kind>]` or `end`.
pub(super) fn practice_test(adb: &Adb, what: &str, stage: &[&str]) -> Result<()> {
    let usage = DeviceError::Usage(
        "cargo device-input practice-test start [stage [n]|copy|drill <kind>]|end",
    );
    match (what, stage) {
        ("start", []) => start(adb, &kept(), "start"),
        ("start", ["copy"]) => start(adb, &kept(), "start:copy"),
        ("start", ["drill", kind]) => {
            let kind = super::plain(kind)?;
            start(adb, &kept(), &format!("start:drill:{kind}"))
        }
        ("start", [stage]) => {
            let stage: u8 = stage.parse().map_err(|_| usage)?;
            start(adb, &kept(), &format!("start:{stage}"))
        }
        ("start", [stage, number]) => {
            let stage = stage.parse::<u8>().ok();
            let number = number.parse::<u32>().ok().filter(|n| *n >= 1);
            let (Some(stage), Some(number)) = (stage, number) else {
                return Err(usage);
            };
            start(adb, &kept(), &format!("start:{stage}:{number}"))
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

/// Whether `name` is a test session's name as the app writes it, `id:sheet:started`: only digits
/// and two colons reach the device's shell.
fn named(name: &str) -> bool {
    let parts: Vec<&str> = name.split(':').collect();
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    matches!(parts[..], [id, sheet, started] if digits(id) && digits(sheet) && digits(started))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_test_sessions_name_reaches_the_device() {
        assert!(named("13:41:1791290000"));
        for bad in ["", "13:41", "13:41:5eed", "13:41:1;rm", "a:41:1", "1:2:3:4"] {
            assert!(!named(bad), "{bad}");
        }
    }
}
