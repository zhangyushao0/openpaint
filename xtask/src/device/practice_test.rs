//! `cargo device-input practice-test start [lesson <skill> [level] | first | copy] | end`
//! (DECISIONS D102): a test lesson (or a test colour copy) beside the author's, for a
//! practice slice to walk on the tablet, and removed afterwards with its sheet, so the author's
//! lessons, copies, records and the path's dots stay as they were.
//!
//! `start` goes to the home (a canvas showing is left by its Home button, nothing drawn), starts
//! the lesson Start would start next (`lesson <skill> [level]`: a lesson of the skill named by its
//! key in a-z and '-' as `line`, `points-in-space` or `key-shift`, at level 1 to 3 or the
//! learner's, the app refusing another name with the list; `first`: the first lesson; `copy`: a
//! colour copy of a pack's
//! picture), beside any open one, and keeps its name (id, sheet, start) in
//! `artifacts/device/practice-test`; it refuses while a name is kept. `end` goes to the home,
//! has the app remove the lesson of the name kept (its record and sheet) and forgets the name; a
//! name the app holds no lesson of (a library restored since) is forgotten and nothing removed.
//! The app matches all three parts, so no other lesson can go.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::adb::{Adb, DeviceError, Result, write_file};
use super::artifacts;
use super::input::{answer, instrument};
use super::state::{Screen, State, tap, until};

/// How long the app may take to get to the home.
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

/// `start [lesson <skill> [level] | first | copy]` or `end`.
pub(super) fn practice_test(adb: &Adb, what: &str, given: &[&str]) -> Result<()> {
    match what {
        "start" => start(adb, &kept(), &command(given)?),
        "end" if given.is_empty() => end(adb, &kept()),
        _ => Err(usage()),
    }
}

fn usage() -> DeviceError {
    DeviceError::Usage(
        "cargo device-input practice-test start [lesson <skill> [level] | first | copy] | end",
    )
}

/// What `start` and the words after it ask the app: `start`, `start:lesson:<skill>[:<level>]`,
/// `start:first`, `start:copy`. A skill goes in a-z and '-' (the app refuses another name with
/// the list), a level 1 to 3.
fn command(given: &[&str]) -> Result<String> {
    let within = |text: &str, most: u8| {
        let number = text.parse::<u8>().ok().filter(|n| (1..=most).contains(n));
        number.ok_or_else(usage)
    };
    Ok(match given {
        [] => "start".to_owned(),
        ["first"] => "start:first".to_owned(),
        ["copy"] => "start:copy".to_owned(),
        ["lesson", skill] => format!("start:lesson:{}", super::plain(skill)?),
        ["lesson", skill, level] => {
            let skill = super::plain(skill)?;
            format!("start:lesson:{skill}:{}", within(level, 3)?)
        }
        _ => return Err(usage()),
    })
}

/// The home, listed: a canvas showing is left first.
fn to_home(adb: &Adb) -> Result<()> {
    if until(adb, WAIT, |s| s.screen != Screen::Starting)?.screen == Screen::Canvas {
        tap(adb, "Home")?;
    }
    until(adb, WAIT, State::listed).map(drop)
}

fn start(adb: &Adb, kept: &Path, command: &str) -> Result<()> {
    if let Ok(name) = std::fs::read_to_string(kept) {
        return Err(failed(format!(
            "test lesson {} is still kept in {}: end it first",
            name.trim(),
            kept.display()
        )));
    }
    to_home(adb)?;
    let output = instrument(adb, &[("practice-test", command.to_owned())])?;
    let said = answer(&output, "practice-test").unwrap_or("no answer");
    let name = said.strip_prefix("started ").filter(|name| named(name));
    let Some(name) = name else {
        return Err(failed(said));
    };
    write_file(kept, name.as_bytes())?;
    println!("started test lesson {name} (kept in {})", kept.display());
    println!("end it with: cargo device-input practice-test end");
    Ok(())
}

fn end(adb: &Adb, kept: &Path) -> Result<()> {
    let name = std::fs::read_to_string(kept)
        .map_err(|_| failed(format!("no test lesson kept in {}", kept.display())))?;
    let name = name.trim();
    if !named(name) {
        return Err(failed(format!(
            "{} holds no test lesson's name",
            kept.display()
        )));
    }
    to_home(adb)?;
    let output = instrument(adb, &[("practice-test", format!("end:{name}"))])?;
    let said = answer(&output, "practice-test").unwrap_or("no answer");
    let gone = said == format!("removed {name}") || said == "no such lesson";
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

/// Whether `name` is a test lesson's name as the app writes it, `id:sheet:started`: only digits
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
    fn only_a_test_lessons_name_reaches_the_device() {
        assert!(named("13:41:1791290000"));
        for bad in ["", "13:41", "13:41:5eed", "13:41:1;rm", "a:41:1", "1:2:3:4"] {
            assert!(!named(bad), "{bad}");
        }
    }

    #[test]
    fn a_start_reaches_the_device_as_the_app_reads_it() {
        for (given, said) in [
            (&[][..], "start"),
            (&["first"][..], "start:first"),
            (&["copy"][..], "start:copy"),
            (
                &["lesson", "points-in-space"][..],
                "start:lesson:points-in-space",
            ),
            (&["lesson", "line", "3"][..], "start:lesson:line:3"),
        ] {
            assert_eq!(command(given).ok().as_deref(), Some(said), "{given:?}");
        }
        for bad in [
            &["lesson"][..],
            &["lesson", "Line"][..],
            &["lesson", "line", "4"][..],
            &["full"][..],
            &["stage", "2"][..],
        ] {
            assert!(command(bad).is_err(), "{bad:?}");
        }
    }
}
