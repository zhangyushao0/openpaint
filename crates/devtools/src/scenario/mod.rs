//! A scenario file: a walk-through written as the user, the screen it runs on, and the steps
//! that replay it with checkpoints. Files live in `tests/scenarios/<name>.json`.
//!
//! ```json
//! {
//!   "walkthrough": ["Open the app.", "Draw a line with the pen; it stays."],
//!   "screen": [640, 400],
//!   "density": 2.0,
//!   "steps": [
//!     { "stroke": { "points": [[100, 200, 0.8], [500, 200, 0.8]] } },
//!     { "tap": { "at": "Undo" } },
//!     { "checkpoint": { "name": "line", "canvas": "0123456789abcdef" } },
//!     { "screenshot": { "name": "line" } }
//!   ]
//! }
//! ```
//!
//! Controls are named (`target.rs`); places on the canvas are window pixels. The steps:
//! `step.rs`.

use std::collections::HashSet;

use serde::{Deserialize, Deserializer};

use crate::error::ScenarioError;
pub use crate::lesson::Lesson;
pub use crate::system::SystemAnswer;

mod step;

pub use step::{Interruption, Lands, Palm, Step, Tool};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The walk-through as the user lives it (not executed; the steps are its replay).
    #[serde(default)]
    pub walkthrough: Vec<String>,
    /// `[width, height]` of the screen in pixels.
    pub screen: [u32; 2],
    /// Physical pixels per UI point.
    pub density: f32,
    /// The system's language the app starts in, as the host's locale: English unless given.
    /// Controls keep their English names in either (D100).
    #[serde(default)]
    pub system_language: SystemLanguage,
    /// Where a library listing no artwork opens: a blank canvas, every scenario's first artwork
    /// (the default), or with `"start": "home"` the home, as the app's first launch does (S122).
    #[serde(default)]
    pub start: StartOn,
    pub steps: Vec<Step>,
}

/// Where a scenario's library listing no artwork opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartOn {
    /// A blank canvas: the scenario's first artwork.
    #[default]
    Canvas,
    /// The home, the app's front door.
    Home,
}

/// The system language a scenario's host reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemLanguage {
    #[default]
    English,
    Chinese,
}

/// An optional hash, as `hex_hash`.
pub(crate) fn optional_hash<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    hex_hash(deserializer).map(Some)
}

impl Scenario {
    /// Parses and validates a scenario file's text.
    pub fn parse(text: &str) -> Result<Self, ScenarioError> {
        let scenario: Self = serde_json::from_str(text).map_err(ScenarioError::Parse)?;
        scenario.validate()?;
        Ok(scenario)
    }

    fn validate(&self) -> Result<(), ScenarioError> {
        if self.screen.contains(&0) {
            return Err(ScenarioError::Invalid("the screen has no pixels".into()));
        }
        if !(self.density.is_finite() && self.density > 0.0) {
            return Err(ScenarioError::Invalid("density must be above 0".into()));
        }
        let mut names = HashSet::new();
        for (index, step) in self.steps.iter().enumerate() {
            let invalid = |reason: &str| ScenarioError::Invalid(format!("step {index}: {reason}"));
            if let Some(reason) = misshapen(step) {
                return Err(invalid(reason));
            }
            if let Some(name) = named(step) {
                if !is_file_name(name) {
                    return Err(invalid("names use a-z, 0-9 and '-' only"));
                }
                if !names.insert(name) {
                    return Err(invalid("the name is already used"));
                }
            }
        }
        Ok(())
    }
}

/// The name a step gives what it records, if it records anything.
fn named(step: &Step) -> Option<&str> {
    match step {
        Step::Checkpoint { name, .. }
        | Step::Preview { name, .. }
        | Step::Press {
            before_lift: Some(name),
            ..
        }
        | Step::Screenshot { name, .. }
        | Step::Exported { name, .. }
        | Step::View { name, .. } => Some(name),
        Step::Video(check) => Some(&check.name),
        _ => None,
    }
}

/// What makes `step` impossible, if anything does (its names apart).
fn misshapen(step: &Step) -> Option<&'static str> {
    match step {
        Step::Stroke { points, .. } if points.is_empty() => {
            Some("a stroke needs at least one point")
        }
        Step::Stroke { tilt, azimuth, .. }
            if !(0.0..=90.0).contains(tilt) || !(-180.0..=180.0).contains(azimuth) =>
        {
            Some("a stroke tilts 0 to 90 degrees, azimuth -180 to 180")
        }
        Step::Press {
            before_lift: None,
            canvas_before_lift: Some(_),
            ..
        } => Some("a canvas before the lift needs its name"),
        Step::Press { path, .. }
            if path.is_empty() || path.windows(2).any(|keys| keys[1].ms() < keys[0].ms()) =>
        {
            Some("a press needs keys whose times never go back")
        }
        Step::Touch { from, to, .. }
            if from.is_empty()
                || from.len() > 5
                || to.as_ref().is_some_and(|to| to.len() != from.len()) =>
        {
            Some("a touch needs 1 to 5 fingers, as many at the end")
        }
        Step::Expect { shown, absent } if shown.is_empty() && absent.is_empty() => {
            Some("an expect step names what is shown or absent")
        }
        Step::Lesson(lesson) if lesson.start().is_err() => {
            Some("a lesson's skill goes by its key, its level 1 to 3; a re-test's unit 1 to 8")
        }
        _ => None,
    }
}

/// Names become file names under `artifacts/`; keep them plain.
pub fn is_file_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// A hash written as 16 hex digits, as the harness prints it.
fn hex_hash<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let text = String::deserialize(deserializer)?;
    if text.len() != 16 {
        return Err(serde::de::Error::custom("a canvas hash is 16 hex digits"));
    }
    u64::from_str_radix(&text, 16).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests;
