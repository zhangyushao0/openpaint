//! `cargo device-input open-file <name>` (S58): an OpenPaint file pushed into the test folder
//! (`cargo device-push`) opened on the tablet as the gallery's Open OpenPaint file… opens a
//! chosen one (S36's import), without the system picker: the debug build reads it through the
//! instrumentation's shell (`DebugControls.openFile`). From the gallery (an artwork open is left
//! first), the new card is told from the author's by the gallery's artwork ids (the one new id,
//! first) and opened from it; its id is printed, for deleting it at the end. Never replaces an
//! artwork. `bench session`'s memory around it (`meminfo`) lives here too.

use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, PACKAGE, Result};
use super::input::{answer, extras, instrument};
use super::push::PUSHED;
use super::smoke_state::{OPEN_CARD, Screen, State, first_is, identify};

/// Why `open-file` stopped.
fn failed(why: impl Into<String>) -> DeviceError {
    DeviceError::Failed {
        command: "device-input open-file".into(),
        output: why.into(),
    }
}

/// How long the import may take (a large file's records are read and written on the store's
/// worker), and how long the app may take to get anywhere else.
const IMPORT: Duration = Duration::from_secs(120);
const WAIT: Duration = Duration::from_secs(90);

/// The pushed file `name` (`a4-180` or `a4-180.openpaint`) opened as a new artwork.
pub(super) fn open_file(adb: &Adb, name: &str) -> Result<()> {
    let stem = name.strip_suffix(".openpaint").unwrap_or(name);
    super::plain(stem)?;
    let path = format!("{PUSHED}/{stem}.openpaint");
    if until(adb, WAIT, |s| s.screen != Screen::Starting)?.screen == Screen::Canvas {
        tap(adb, "Gallery")?;
    }
    let before = until(adb, WAIT, listed)?;
    before.every_card_shown().map_err(failed)?;
    let started = Instant::now();
    let output = instrument(adb, &[("open-file", path.clone())])?;
    let opening = answer(&output, "opening").unwrap_or("no answer");
    if !opening.starts_with("opening") {
        return Err(failed(format!("{path}: {opening}")));
    }
    let added = |s: &State| listed(s) && s.cards.len() > before.cards.len();
    let after = until(adb, IMPORT, added)?;
    let id = identify(&before.cards, &after).map_err(failed)?;
    let imported = started.elapsed().as_secs_f64();
    first_is(id, &after).map_err(failed)?;
    tap(adb, &format!("{OPEN_CARD} #1"))?;
    until(adb, WAIT, |s| {
        s.screen == Screen::Canvas && s.shows("Brushes")
    })?;
    println!(
        "opened {path} as artwork {id} (imported in {imported:.1} s, open after {:.1} s); the \
         gallery held {:?}",
        started.elapsed().as_secs_f64(),
        before.cards
    );
    Ok(())
}

fn listed(state: &State) -> bool {
    state.screen == Screen::Gallery && state.listed
}

fn tap(adb: &Adb, name: &str) -> Result<()> {
    let mut words = extras(&["tap", name])?;
    words.push(("within", "3000".to_owned()));
    instrument(adb, &words).map(drop)
}

fn state(adb: &Adb) -> Result<State> {
    let output = instrument(adb, &extras(&["state"])?)?;
    let state = answer(&output, "state").ok_or_else(|| failed("the app gave no state"))?;
    State::parse(state).map_err(failed)
}

/// Asks what the app shows until `done` holds, at most `limit`.
fn until(adb: &Adb, limit: Duration, done: impl Fn(&State) -> bool) -> Result<State> {
    let started = Instant::now();
    loop {
        let state = state(adb)?;
        if done(&state) {
            return Ok(state);
        }
        if started.elapsed() >= limit {
            return Err(failed(format!(
                "waited {} s; the app shows {:?}, cards {:?}",
                limit.as_secs(),
                state.screen,
                state.cards
            )));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The app's memory as `dumpsys meminfo` sums it (its App Summary: Java and native heap,
/// Graphics, total PSS), one line.
pub(super) fn meminfo(adb: &Adb) -> Result<String> {
    let dump = adb.shell_within(
        &["dumpsys", "-t", "10", "meminfo", PACKAGE],
        Duration::from_secs(30),
    )?;
    let keys = ["Java Heap:", "Native Heap:", "Graphics:", "TOTAL PSS:"];
    let lines: Vec<String> = dump
        .lines()
        .map(str::trim)
        .filter(|line| keys.iter().any(|key| line.starts_with(key)))
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    if lines.is_empty() {
        return Err(DeviceError::Failed {
            command: "dumpsys meminfo".into(),
            output: "no App Summary (is the app running?)".into(),
        });
    }
    Ok(format!("meminfo (KB): {}", lines.join("; ")))
}
