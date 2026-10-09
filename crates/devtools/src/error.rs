//! `ScenarioError`: why a scenario could not be read or run. A failed checkpoint is not an
//! error; it is a result in the report.

use std::fmt;

use openpaint_runtime::{RuntimeError, Unresolved};

#[derive(Debug)]
pub enum ScenarioError {
    /// The file is not valid scenario JSON; the message carries line and column.
    Parse(serde_json::Error),
    /// The file parses but says something impossible.
    Invalid(String),
    Runtime(RuntimeError),
    /// A step needs something the app has not produced yet (no frame, no canvas).
    Missing(&'static str),
    /// A step names a control the screen does not show as one now; `available` lists every
    /// name it could use.
    Control {
        name: String,
        reason: Unresolved,
        available: Vec<String>,
    },
    /// An `expect` step found the screen otherwise (S47): `what` is wrong; `available` lists
    /// every name on screen.
    Unexpected {
        what: String,
        available: Vec<String>,
    },
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "scenario file: {error}"),
            Self::Invalid(reason) => write!(f, "scenario file: {reason}"),
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Missing(what) => write!(f, "{what}"),
            Self::Control {
                name,
                reason,
                available,
            } => {
                match reason {
                    Unresolved::Missing => write!(f, "no control is named {name:?} now")?,
                    Unresolved::Shared { count } => write!(
                        f,
                        "{count} controls are named {name:?}: say \"{name} #1\" to \"{name} #{count}\""
                    )?,
                }
                write!(f, "; the screen shows: {}", available.join(", "))
            }
            Self::Unexpected { what, available } => write!(
                f,
                "expected otherwise: {what}; the screen shows: {}",
                available.join(", ")
            ),
        }
    }
}

impl std::error::Error for ScenarioError {}

impl From<RuntimeError> for ScenarioError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}
