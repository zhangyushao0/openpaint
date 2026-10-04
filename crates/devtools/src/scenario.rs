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
//! Controls are named (`target.rs`); places on the canvas are window pixels.

use std::collections::HashSet;

use serde::{Deserialize, Deserializer};

use crate::error::ScenarioError;
pub use crate::system::SystemAnswer;
use crate::target::{Key, Spot};

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
    pub steps: Vec<Step>,
}

/// One thing the user does, or one thing the runner looks at.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    /// Touch down and lift at one spot.
    Tap {
        at: Spot,
        #[serde(default = "Tool::finger")]
        tool: Tool,
    },
    /// A straight drag at full pressure.
    Drag {
        from: Spot,
        to: Spot,
        #[serde(default = "Tool::finger")]
        tool: Tool,
    },
    /// A pen stroke through `[x, y, pressure]` points, pressure interpolated between them.
    Stroke {
        points: Vec<[f32; 3]>,
        /// Where the pen leaves the glass, reported as Android does: one more sample with
        /// pressure 0. Without it the last point is the lift.
        #[serde(default)]
        lift: Option<[f32; 2]>,
        /// A hand resting on the glass around the stroke.
        #[serde(default)]
        palm: Option<Palm>,
    },
    /// One contact through keys (a place and `ms` from touching down) at `pressure` (default
    /// full): it moves straight between keys in their time and rests where two keys share a
    /// place (holding still), sampled every 4 ms.
    Press {
        path: Vec<Key>,
        #[serde(default = "Tool::finger")]
        tool: Tool,
        #[serde(default = "full_pressure")]
        pressure: f32,
        /// A screenshot of this name just before the contact lifts.
        #[serde(default)]
        before_lift: Option<String>,
    },
    /// The pen hovers just above the glass at `at`, touching nothing; it stays there until the
    /// next contact.
    Hover { at: Spot },
    /// Fingers touch down at `from`, move together straight to `to` (default: stay) over `ms`
    /// (default 64), and lift: two fingers apart or together pinch, turned they rotate, moved
    /// alike they pan; fingers that stay tap.
    Touch {
        from: Vec<Spot>,
        #[serde(default)]
        to: Option<Vec<Spot>>,
        #[serde(default = "default_touch_ms")]
        ms: u32,
    },
    /// The view must be at `zoom` (relative to fit, within 1 %) and `rotation` (degrees,
    /// within half a degree), and with `pan` (points, within one) when given.
    View {
        name: String,
        zoom: f32,
        rotation: f32,
        #[serde(default)]
        pan: Option<[f32; 2]>,
    },
    /// Text committed by the keyboard into the field that has the focus.
    Type(String),
    /// The system's back button.
    Back,
    /// Frames with no input.
    Wait { frames: u32 },
    /// The app process ends and starts again with the same screen and the same library, as
    /// after leaving the app and coming back once the system has closed it.
    Restart,
    /// From now on every storage call fails, as on a full or removed disk.
    StorageFails,
    /// Storage works again.
    StorageWorks,
    /// The canvas pixels (the artwork without the UI) must hash to `canvas`. The canvas, not
    /// the screen, because only the evaluator's pixels are the same on every GPU backend.
    Checkpoint {
        name: String,
        #[serde(deserialize_with = "hex_hash")]
        canvas: u64,
    },
    /// The screen, kept for looking at; never compared. Taken once every motion has settled,
    /// or, with `after_ms`, that long after the last input step ended, while it may still move
    /// (the runner's clock is fixed, so it moves alike every run).
    Screenshot {
        name: String,
        #[serde(default)]
        after_ms: Option<u32>,
    },
    /// The user answers the system picker, dialog or share sheet the app asked for.
    System(SystemAnswer),
    /// The file the last export wrote, decoded: a PNG must equal the canvas pixel for pixel, a
    /// JPEG come close to it.
    Exported { name: String },
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

fn default_touch_ms() -> u32 {
    64
}

fn full_pressure() -> f32 {
    1.0
}

/// A hand on the glass at `at` (window pixels), a touch of its own beside the pen.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palm {
    pub at: [f32; 2],
    pub lands: Lands,
}

/// When the hand touches the glass, relative to the pen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lands {
    /// Before the pen touches down; it lifts after the pen.
    Before,
    /// After the pen touched down; it lifts before the pen.
    During,
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
                Step::Stroke { points, .. } if points.is_empty() => {
                    return Err(invalid("a stroke needs at least one point"));
                }
                Step::Press { path, .. }
                    if path.is_empty()
                        || path.windows(2).any(|keys| keys[1].ms() < keys[0].ms()) =>
                {
                    return Err(invalid("a press needs keys whose times never go back"));
                }
                Step::Touch { from, to, .. }
                    if from.is_empty()
                        || from.len() > 5
                        || to.as_ref().is_some_and(|to| to.len() != from.len()) =>
                {
                    return Err(invalid("a touch needs 1 to 5 fingers, as many at the end"));
                }
                Step::Checkpoint { name, .. }
                | Step::Press {
                    before_lift: Some(name),
                    ..
                }
                | Step::Screenshot { name, .. }
                | Step::Exported { name }
                | Step::View { name, .. } => {
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
                { "stroke": { "points": [[1, 2, 0.5]], "lift": [3, 4],
                              "palm": { "at": [5, 6], "lands": "during" } } },
                { "wait": { "frames": 2 } },
                "restart",
                { "checkpoint": { "name": "a", "canvas": "1f94cbfbd4b11e02" } },
                { "screenshot": { "name": "b" } },
                "storage_fails",
                "storage_works",
                { "type": "Harbour" },
                "back",
                { "system": "choose_file" },
                { "exported": { "name": "png" } }
            ]
        }"#;
        let scenario = Scenario::parse(text).expect("parses");
        assert_eq!(scenario.walkthrough, ["Open the app."]);
        assert_eq!(scenario.steps.len(), 14);
        assert_eq!(scenario.steps[12], Step::System(SystemAnswer::ChooseFile));
        assert_eq!(scenario.steps[13], Step::Exported { name: "png".into() });
        assert_eq!(scenario.steps[8], Step::StorageFails);
        assert_eq!(scenario.steps[9], Step::StorageWorks);
        assert_eq!(scenario.steps[10], Step::Type("Harbour".into()));
        assert_eq!(scenario.steps[11], Step::Back);
        assert_eq!(
            scenario.steps[3],
            Step::Stroke {
                points: vec![[1.0, 2.0, 0.5]],
                lift: Some([3.0, 4.0]),
                palm: Some(Palm {
                    at: [5.0, 6.0],
                    lands: Lands::During
                }),
            }
        );
        assert_eq!(
            scenario.steps[0],
            Step::Tap {
                at: Spot::Point([10.0, 20.0]),
                tool: Tool::Finger
            }
        );
        assert_eq!(scenario.steps[5], Step::Restart);
        assert_eq!(
            scenario.steps[6],
            Step::Checkpoint {
                name: "a".into(),
                canvas: 0x1f94_cbfb_d4b1_1e02
            }
        );
    }

    #[test]
    fn finger_and_view_steps_parse() {
        let steps = r#"
            { "touch": { "from": [[1, 2], [3, 4]], "to": [[0, 2], [5, 4]], "ms": 200 } },
            { "touch": { "from": [[1, 2], [3, 4], [5, 6]] } },
            { "view": { "name": "zoomed", "zoom": 2.5, "rotation": -30 } }"#;
        let scenario = with_steps(steps).expect("parses");
        let points = |a: [f32; 2], b: [f32; 2]| vec![Spot::Point(a), Spot::Point(b)];
        let two = Step::Touch {
            from: points([1.0, 2.0], [3.0, 4.0]),
            to: Some(points([0.0, 2.0], [5.0, 4.0])),
            ms: 200,
        };
        assert_eq!(scenario.steps[0], two);
        assert!(matches!(
            &scenario.steps[1],
            Step::Touch {
                to: None,
                ms: 64,
                ..
            }
        ));
        let view = Step::View {
            name: "zoomed".into(),
            zoom: 2.5,
            rotation: -30.0,
            pan: None,
        };
        assert_eq!(scenario.steps[2], view);
        let uneven = r#"{ "touch": { "from": [[1, 2]], "to": [[1, 2], [3, 4]] } }"#;
        assert!(matches!(with_steps(uneven), Err(ScenarioError::Invalid(_))));
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
