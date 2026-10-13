//! `cargo device-smoke` (S52): the installed debug build walked through its main features in
//! about 20 s, one line per step with pass or why it failed, on a test artwork it creates and
//! deletes; nothing is built or installed (`device-check` installs).
//!
//! The steps go through the same instrumentation as `device-input` (taps by name, a pen path,
//! a drag from a named control), each waiting up to `WITHIN` for its control, and ask the debug
//! build what it shows between them (`device-input state`, `smoke_state.rs`). The device steps
//! are this list, not a scenario file: xtask depends on nothing (ARCHITECTURE section 2), and a
//! device step names screen pixels and checks the device, not canvas hashes.
//!
//! The author's artworks are never touched: the gallery's artwork ids are read before the test
//! artwork is made; it is the one new id, first (changed last) once its canvas is left, and it
//! is opened and deleted as card #1 only while that still holds and every card shows on the
//! screen (no search hiding one, none off it); otherwise the smoke stops with a screenshot
//! (`artifacts/device/smoke-failed.png`). It does change the toolbox, as drawing does: the
//! eraser stays chosen and Recent lists the brushes it drew with.

use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, Result};
use super::check::{app_errors, fatal, screenshot, wake};
use super::clean_on;
use super::input::{answer, extras, instrument};
use super::smoke_state::{
    DELETE_CARD, OPEN_CARD, Screen, State, WAIT, failed, first_is, focus_until, gralloc, identify,
    installed, path, point, shared,
};
use crate::android::ACTIVITY;

/// What the steps after the app is up should take at most (ROADMAP S52).
const TARGET: Duration = Duration::from_secs(20);
/// How long a step waits for a control to show before it fails, in milliseconds.
const WITHIN: &str = "3000";
/// How long the app may take to reach the gallery from what it showed.
const UP: Duration = Duration::from_secs(90);

/// One step; the places are fractions of where the canvas fits.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    Gallery,
    Create,
    Stroke(&'static str, &'static [[f32; 2]]),
    UndoRedo,
    Layer,
    Fill([f32; 2]),
    Export,
    Leave,
    Reopen,
    Delete,
    Clean,
}

const STEPS: [(&str, Step); 13] = [
    (
        "the gallery: every card on screen, their ids",
        Step::Gallery,
    ),
    ("New artwork, Screen size: its canvas", Step::Create),
    ("Brush: a stroke", Step::Stroke("Brushes", &BRUSH)),
    (
        "Smudge: a stroke across it",
        Step::Stroke("Smudge", &[[0.37, 0.22], [0.39, 0.5]]),
    ),
    (
        "Eraser: a stroke across it",
        Step::Stroke("Eraser", &[[0.53, 0.22], [0.51, 0.5]]),
    ),
    ("Undo, then Redo", Step::UndoRedo),
    ("Layers: Layer 2 added, then deleted", Step::Layer),
    (
        "a fill dropped from the colour button",
        Step::Fill([0.45, 0.8]),
    ),
    (
        "Share PNG…: the file in the share cache, the sheet closed",
        Step::Export,
    ),
    ("left: the newest card is the test artwork", Step::Leave),
    ("reopened from its card", Step::Reopen),
    (
        "left again, deleted as card #1, the others all there",
        Step::Delete,
    ),
    ("the agent's test files cleaned", Step::Clean),
];

/// The brush's wave across the canvas's upper middle.
const BRUSH: [[f32; 2]; 6] = [
    [0.25, 0.4],
    [0.33, 0.3],
    [0.41, 0.42],
    [0.49, 0.3],
    [0.57, 0.42],
    [0.65, 0.32],
];

struct Smoke {
    adb: Adb,
    /// The gallery's artworks before the test artwork was made.
    before: Vec<u64>,
    /// The test artwork, once the gallery listed it.
    ours: Option<u64>,
    /// Where the canvas fits on the screen, in pixels: left, top, width, height.
    fit: [f32; 4],
    /// A test artwork this run made is in the library.
    made: bool,
}

pub fn smoke(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(DeviceError::Usage("cargo device-smoke"));
    }
    let adb = Adb::connected()?;
    wake(&adb)?;
    println!("smoke on {}", installed(&adb)?);
    let since = adb.shell(&["date", "+%s"])?.trim().to_owned();
    let launched = Instant::now();
    adb.shell(&["am", "start", "-W", "-n", ACTIVITY])?;
    let mut smoke = Smoke {
        adb,
        before: Vec::new(),
        ours: None,
        fit: [0.0; 4],
        made: false,
    };
    // Up means in the gallery: leaving an artwork the author left open waits for its canvas to
    // be complete (its gallery picture), seconds on a dense one, which is not the smoke's.
    if smoke
        .until(WAIT, |state| state.screen != Screen::Starting)?
        .screen
        == Screen::Canvas
    {
        smoke.tap("Gallery")?;
    }
    smoke.until(UP, |s| s.screen == Screen::Gallery && s.listed)?;
    println!(
        "the app is up in the gallery after {:.1} s",
        launched.elapsed().as_secs_f64()
    );
    let started = Instant::now();
    let mut result = Ok(());
    for (label, step) in STEPS {
        result = smoke.step(label, |smoke| smoke.run(step));
        if result.is_err() {
            break;
        }
    }
    let result = result.and_then(|()| {
        let label = "the app's log: no panic, no errors but Gralloc's";
        smoke.step(label, |smoke| smoke.log(&since))
    });
    let seconds = started.elapsed().as_secs_f64();
    if let Err(error) = result {
        let _ = screenshot(&smoke.adb, "smoke-failed");
        if smoke.made {
            println!("the test artwork this run made may still be in the gallery (newest card)");
        }
        return Err(error);
    }
    let over = if seconds > TARGET.as_secs_f64() {
        " (over the target)"
    } else {
        ""
    };
    let target = TARGET.as_secs();
    println!("smoke: pass, {seconds:.1} s of steps, target {target} s{over}");
    Ok(())
}

impl Smoke {
    /// Runs `action` as the step `label` and prints its line.
    fn step(&mut self, label: &str, action: impl FnOnce(&mut Self) -> Result<()>) -> Result<()> {
        let started = Instant::now();
        let result = action(self);
        let seconds = started.elapsed().as_secs_f64();
        match &result {
            Ok(()) => println!("  pass {seconds:5.2} s  {label}"),
            Err(error) => println!("  FAIL {seconds:5.2} s  {label}: {error}"),
        }
        result
    }

    fn run(&mut self, step: Step) -> Result<()> {
        match step {
            Step::Gallery => self.gallery(),
            Step::Create => self.create(),
            Step::Stroke(tool, waypoints) => {
                self.tap(tool)?;
                let path = path(self.fit, waypoints);
                self.ask(&["pen", &path, "0.7"], false).map(drop)
            }
            Step::UndoRedo => {
                self.tap("Undo")?;
                self.tap("Redo")
            }
            Step::Layer => self.layer(),
            Step::Fill(at) => {
                let [x, y] = point(self.fit, at);
                let to = format!("{},{}", x.round(), y.round());
                self.ask(&["drag", "Colour", &to, "240"], false)?;
                self.ask(&["find", "Continue filling"], true).map(drop)
            }
            Step::Export => self.export(),
            Step::Leave => self.leave(),
            Step::Reopen => self.reopen(),
            Step::Delete => self.delete(),
            Step::Clean => clean_on(&self.adb).map(drop),
        }
    }

    /// One instrumentation run of the `device-input` line `words`, waiting up to `WITHIN` for
    /// the control it names when `within`.
    fn ask(&self, words: &[&str], within: bool) -> Result<String> {
        let mut extras = extras(words)?;
        if within {
            extras.push(("within", WITHIN.to_owned()));
        }
        instrument(&self.adb, &extras)
    }

    fn tap(&self, name: &str) -> Result<()> {
        self.ask(&["tap", name], true).map(drop)
    }

    fn state(&self) -> Result<State> {
        let output = self.ask(&["state"], false)?;
        let state = answer(&output, "state").ok_or_else(|| failed("the app gave no state"))?;
        State::parse(state).map_err(failed)
    }

    /// Asks what the app shows until `done` holds, at most `limit`.
    fn until(&self, limit: Duration, done: impl Fn(&State) -> bool) -> Result<State> {
        let started = Instant::now();
        loop {
            let state = self.state()?;
            if done(&state) {
                return Ok(state);
            }
            if started.elapsed() >= limit {
                return Err(failed(format!(
                    "waited {} s; the app shows {:?} {:?}",
                    limit.as_secs(),
                    state.screen,
                    state.names
                )));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn listed(&self) -> Result<State> {
        self.until(WAIT, |state| {
            state.screen == Screen::Gallery && state.listed
        })
    }

    /// The gallery the app is up in: the author's artworks as found.
    fn gallery(&mut self) -> Result<()> {
        let state = self.listed()?;
        state.every_card_shown().map_err(failed)?;
        self.before = state.cards;
        Ok(())
    }

    fn create(&mut self) -> Result<()> {
        self.tap("New artwork")?;
        self.tap("Screen size")?;
        self.made = true;
        let state = self.until(WAIT, |s| s.screen == Screen::Canvas && s.shows("Brushes"))?;
        self.fit = state.fit.ok_or_else(|| failed("no canvas area"))?;
        Ok(())
    }

    fn layer(&mut self) -> Result<()> {
        self.tap("Layers")?;
        self.tap("Add layer")?;
        self.ask(&["find", "Layer 2"], true)?;
        self.tap("Delete layer")?;
        self.until(WAIT, |state| !state.shows("Layer 2"))?;
        self.tap("Layers")
    }

    /// Share PNG… writes the canvas into the app's share cache and opens the system's share
    /// sheet, which back closes (nothing is sent anywhere); Save PNG… would need the system's
    /// save dialog driven.
    fn export(&mut self) -> Result<()> {
        self.tap("Actions")?;
        self.tap("Share PNG…")?;
        focus_until(&self.adb, |focus| focus.contains("Chooser"))?;
        shared(&self.adb, self.fit)?;
        self.adb.shell(&["input", "keyevent", "KEYCODE_BACK"])?;
        focus_until(&self.adb, |focus| focus.contains("dev.openpaint/"))
    }

    fn leave(&mut self) -> Result<()> {
        self.tap("Gallery")?;
        let state = self.listed()?;
        self.ours = Some(identify(&self.before, &state).map_err(failed)?);
        Ok(())
    }

    fn ours(&self) -> Result<u64> {
        self.ours.ok_or_else(|| failed("no test artwork"))
    }

    fn reopen(&mut self) -> Result<()> {
        first_is(self.ours()?, &self.state()?).map_err(failed)?;
        self.tap(&format!("{OPEN_CARD} #1"))?;
        self.ask(&["find", "Brushes"], true).map(drop)
    }

    fn delete(&mut self) -> Result<()> {
        let ours = self.ours()?;
        self.tap("Gallery")?;
        first_is(ours, &self.listed()?).map_err(failed)?;
        self.tap(&format!("{DELETE_CARD} #1"))?;
        self.tap("Delete")?;
        let gone = |state: &State| {
            state.screen == Screen::Gallery && state.listed && !state.cards.contains(&ours)
        };
        let mut cards = self.until(WAIT, gone)?.cards;
        self.made = false;
        let mut before = self.before.clone();
        cards.sort_unstable();
        before.sort_unstable();
        if cards != before {
            return Err(failed(format!(
                "the gallery lists {cards:?}, not {before:?}"
            )));
        }
        Ok(())
    }

    fn log(&self, since: &str) -> Result<()> {
        let (pid, lines) = app_errors(&self.adb, since)?;
        let fatal = fatal(&lines);
        if !fatal.is_empty() {
            return Err(DeviceError::Panic(fatal));
        }
        if pid.is_none() {
            return Err(DeviceError::Panic(vec!["the app is not running".into()]));
        }
        let others: Vec<&String> = lines.iter().filter(|line| !gralloc(line)).collect();
        if !others.is_empty() {
            return Err(failed(format!("error lines: {others:#?}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_make_before_they_draw_and_delete_last() {
        let at = |wanted: Step| STEPS.iter().position(|(_, step)| *step == wanted);
        let strokes: Vec<usize> = (0..STEPS.len())
            .filter(|&i| matches!(STEPS[i].1, Step::Stroke(..)))
            .collect();
        assert_eq!(strokes.len(), 3, "a stroke of each tool");
        let order = [
            Step::Gallery,
            Step::Create,
            Step::Export,
            Step::Leave,
            Step::Reopen,
            Step::Delete,
            Step::Clean,
        ];
        let places: Vec<usize> = order.iter().map(|&s| at(s).expect("a step")).collect();
        assert!(
            places.windows(2).all(|pair| pair[0] < pair[1]),
            "{places:?}"
        );
        assert!(strokes.iter().all(|&i| i > places[1] && i < places[3]));
        let mut labels: Vec<&str> = STEPS.iter().map(|(label, _)| *label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), STEPS.len());
        for (_, step) in STEPS {
            let places: &[[f32; 2]] = match &step {
                Step::Stroke(_, path) => path,
                Step::Fill(at) => std::slice::from_ref(at),
                _ => &[],
            };
            // On the canvas, clear of the side bar, the islands and the right island's panels.
            assert!(places.iter().flatten().all(|f| (0.2..=0.8).contains(f)));
        }
    }
}
