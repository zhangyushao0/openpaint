//! `ScenarioError`: why a scenario could not be read or run. A failed checkpoint is not an
//! error; it is a result in the report.

use std::fmt;

use openpaint_runtime::RuntimeError;

#[derive(Debug)]
pub enum ScenarioError {
    /// The file is not valid scenario JSON; the message carries line and column.
    Parse(serde_json::Error),
    /// The file parses but says something impossible.
    Invalid(String),
    Runtime(RuntimeError),
    /// A step needs something the app has not produced yet (no frame, no canvas).
    Missing(&'static str),
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "scenario file: {error}"),
            Self::Invalid(reason) => write!(f, "scenario file: {reason}"),
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Missing(what) => write!(f, "{what}"),
        }
    }
}

impl std::error::Error for ScenarioError {}

impl From<RuntimeError> for ScenarioError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}
