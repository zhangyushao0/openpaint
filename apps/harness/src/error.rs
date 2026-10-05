//! `HarnessError`: why a harness command failed. Printed once by `main`.

use std::time::Duration;
use std::{fmt, io};

use openpaint_core::CoreError;
use openpaint_devtools::ScenarioError;
use openpaint_render::RenderError;
use openpaint_runtime::{PortError, RuntimeError};

#[derive(Debug)]
pub enum HarnessError {
    Core(CoreError),
    Render(RenderError),
    Runtime(RuntimeError),
    Io(io::Error),
    Png(png::EncodingError),
    /// Something the command needs was not produced.
    Missing(&'static str),
    /// The written image's hash is not the golden value.
    Golden {
        expected: u64,
        actual: u64,
    },
    /// The slowest frame took longer than the budget.
    Budget {
        slowest: Duration,
        budget: Duration,
    },
    Scenario(ScenarioError),
    /// The library refused a call.
    Port(PortError),
    /// A scenario ran to the end but some checkpoints did not match.
    Checkpoints {
        failed: usize,
        total: usize,
    },
    /// Steps touched controls by window pixels instead of naming them.
    Unnamed(usize),
    /// `scenario --all` ran every scenario and some failed.
    Scenarios {
        failed: usize,
        total: usize,
    },
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(f, "{error}"),
            Self::Render(error) => write!(f, "{error}"),
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "file error: {error}"),
            Self::Png(error) => write!(f, "PNG encoding error: {error}"),
            Self::Missing(what) => write!(f, "{what}"),
            Self::Golden { expected, actual } => {
                write!(f, "hash {actual:016x} differs from golden {expected:016x}")
            }
            Self::Budget { slowest, budget } => {
                write!(f, "slowest frame {slowest:?} is over the {budget:?} budget")
            }
            Self::Scenario(error) => write!(f, "{error}"),
            Self::Port(error) => write!(f, "library: {error}"),
            Self::Checkpoints { failed, total } => {
                write!(f, "{failed} of {total} checkpoints failed")
            }
            Self::Unnamed(count) => {
                write!(f, "{count} steps touch a control by pixels; name it")
            }
            Self::Scenarios { failed, total } => {
                write!(f, "{failed} of {total} scenarios failed")
            }
        }
    }
}

impl From<CoreError> for HarnessError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl From<RenderError> for HarnessError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl From<RuntimeError> for HarnessError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ScenarioError> for HarnessError {
    fn from(error: ScenarioError) -> Self {
        Self::Scenario(error)
    }
}

impl From<io::Error> for HarnessError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<png::EncodingError> for HarnessError {
    fn from(error: png::EncodingError) -> Self {
        Self::Png(error)
    }
}
