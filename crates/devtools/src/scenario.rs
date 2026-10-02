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
//!     { "checkpoint": { "name": "line", "canvas": "0123456789abcdef" } },
//!     { "screenshot": { "name": "line" } }
//!   ]
//! }
//! ```
//!
//! Positions are window pixels, as the host delivers them; the top bar is part of the screen.

use std::collections::HashSet;

use serde::{Deserialize, Deserializer};

use crate::error::ScenarioError;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The walk-through as the user lives it: goal, gestures, what appears, success, failure.
    /// Not executed; the steps are its replay.
    #[serde(default)]
    pub walkthrough: Vec<String>,
    /// `[width, height]` of the screen in pixels.
    pub screen: [u32; 2],
    /// Physical pixels per UI point.
    pub density: f32,
    pub steps: Vec<Step>,
}

/// One thing the user does, or one thing the runner looks at.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    /// Touch down and lift at one spot.
    Tap {
        at: [f32; 2],
        #[serde(default = "Tool::finger")]
        tool: Tool,
    },
    /// A straight drag at full pressure.
    Drag {
        from: [f32; 2],
        to: [f32; 2],
        #[serde(default = "Tool::finger")]
        tool: Tool,
    },
    /// A pen stroke through `[x, y, pressure]` points, pressure interpolated between them.
    Stroke { points: Vec<[f32; 3]> },
    /// Frames with no input.
    Wait { frames: u32 },
    /// The app process ends and starts again with the same screen.
    Restart,
    /// The canvas pixels (the artwork without the UI) must hash to `canvas`. The canvas, not
    /// the screen, because only the evaluator's pixels are the same on every GPU backend.
    Checkpoint {
        name: String,
        #[serde(deserialize_with = "hex_hash")]
        canvas: u64,
    },
    /// The last frame's screen, kept for looking at; never compared.
    Screenshot { name: String },
}

/// What touches the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    Finger,
    Stylus,
}

impl Tool {
    fn finger() -> Self {
        Self::Finger
    }
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
            match step {
                Step::Stroke { points } if points.is_empty() => {
                    return Err(invalid("a stroke needs at least one point"));
                }
                Step::Checkpoint { name, .. } | Step::Screenshot { name } => {
                    if !is_file_name(name) {
                        return Err(invalid("names use a-z, 0-9 and '-' only"));
                    }
                    if !names.insert(name.as_str()) {
                        return Err(invalid("the name is already used"));
                    }
                }
                _ => {}
            }
        }
        Ok(())
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
mod tests {
    use super::*;

    #[test]
    fn every_step_kind_parses() {
        let text = r#"{
            "walkthrough": ["Open the app."],
            "screen": [640, 400],
            "density": 2,
            "steps": [
                { "tap": { "at": [10, 20] } },
                { "drag": { "from": [1, 2], "to": [3, 4], "tool": "stylus" } },
                { "stroke": { "points": [[100, 200, 0.5]] } },
                { "wait": { "frames": 2 } },
                "restart",
                { "checkpoint": { "name": "a", "canvas": "1f94cbfbd4b11e02" } },
                { "screenshot": { "name": "b" } }
            ]
        }"#;
        let scenario = Scenario::parse(text).expect("parses");
        assert_eq!(scenario.walkthrough, ["Open the app."]);
        assert_eq!(scenario.steps.len(), 7);
        assert_eq!(
            scenario.steps[0],
            Step::Tap {
                at: [10.0, 20.0],
                tool: Tool::Finger
            }
        );
        assert_eq!(scenario.steps[4], Step::Restart);
        assert_eq!(
            scenario.steps[5],
            Step::Checkpoint {
                name: "a".into(),
                canvas: 0x1f94_cbfb_d4b1_1e02
            }
        );
    }

    fn with_steps(steps: &str) -> Result<Scenario, ScenarioError> {
        Scenario::parse(&format!(
            r#"{{ "screen": [8, 8], "density": 1, "steps": [{steps}] }}"#
        ))
    }

    #[test]
    fn unknown_steps_and_fields_are_refused_with_a_position() {
        let error = with_steps(r#"{ "jump": {} }"#).expect_err("unknown step");
        assert!(error.to_string().contains("line 1"), "{error}");
        assert!(with_steps(r#"{ "tap": { "at": [1, 2], "force": 1 } }"#).is_err());
    }

    #[test]
    fn hashes_are_sixteen_hex_digits() {
        assert!(with_steps(r#"{ "checkpoint": { "name": "a", "canvas": "1f94" } }"#).is_err());
        let bad = r#"{ "checkpoint": { "name": "a", "canvas": "1f94cbfbd4b11e0g" } }"#;
        assert!(with_steps(bad).is_err());
    }

    #[test]
    fn names_are_plain_and_unique() {
        let upper = r#"{ "screenshot": { "name": "Final" } }"#;
        assert!(matches!(with_steps(upper), Err(ScenarioError::Invalid(_))));
        let twice = r#"{ "screenshot": { "name": "a" } }, { "screenshot": { "name": "a" } }"#;
        assert!(matches!(with_steps(twice), Err(ScenarioError::Invalid(_))));
        let empty = r#"{ "stroke": { "points": [] } }"#;
        assert!(matches!(with_steps(empty), Err(ScenarioError::Invalid(_))));
    }
}
