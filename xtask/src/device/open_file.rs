//! `cargo device-input open-file <name>`: an OpenPaint file pushed into the test folder
//! (`cargo device-push`) opened on the tablet as the gallery's Open OpenPaint file… opens a
//! chosen one, without the system picker: the debug build reads it through the
//! instrumentation's shell (`DebugControls.openFile`). From the gallery (an artwork open is left
//! first), the new card is told from the author's by the gallery's artwork ids (the one new id,
//! first) and opened from it; its id is printed, for deleting it at the end. Never replaces an
//! artwork. A pushed practice pack (`<name>.pack`) is imported the same way, as Import practice
//! pack… imports a chosen one (the restore takes it away again, `library.rs`).

use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, Result};
use super::input::{answer, instrument};
use super::push::PUSHED;
use super::state::{OPEN_CARD, Screen, State, first_is, identify, tap, until};

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

/// The pushed practice pack `stem.pack` imported as the practice options' Import practice pack…
/// imports a chosen one, from the gallery; no card changes.
fn import_pack(adb: &Adb, stem: &str) -> Result<()> {
    super::plain(stem)?;
    let path = format!("{PUSHED}/{stem}.pack");
    if until(adb, WAIT, |s| s.screen != Screen::Starting)?.screen == Screen::Canvas {
        tap(adb, "Gallery")?;
    }
    until(adb, WAIT, State::listed)?;
    let started = Instant::now();
    let output = instrument(adb, &[("open-file", path.clone())])?;
    let said = answer(&output, "opening").unwrap_or("no answer");
    if !said.starts_with("opening") {
        return Err(failed(format!("{path}: {said}")));
    }
    println!(
        "imported the practice pack {path} in {:.1} s (the gallery's notice says how it went)",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// The pushed file `name` (`a4-180` or `a4-180.openpaint`) opened as a new artwork; a pushed
/// practice pack (`name.pack`) imported.
pub(super) fn open_file(adb: &Adb, name: &str) -> Result<()> {
    if let Some(stem) = name.strip_suffix(".pack") {
        return import_pack(adb, stem);
    }
    let stem = name.strip_suffix(".openpaint").unwrap_or(name);
    super::plain(stem)?;
    let path = format!("{PUSHED}/{stem}.openpaint");
    if until(adb, WAIT, |s| s.screen != Screen::Starting)?.screen == Screen::Canvas {
        tap(adb, "Gallery")?;
    }
    let before = until(adb, WAIT, State::listed)?;
    before.every_card_shown().map_err(failed)?;
    let started = Instant::now();
    let output = instrument(adb, &[("open-file", path.clone())])?;
    let opening = answer(&output, "opening").unwrap_or("no answer");
    if !opening.starts_with("opening") {
        return Err(failed(format!("{path}: {opening}")));
    }
    let added = |s: &State| s.listed() && s.cards.len() > before.cards.len();
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
