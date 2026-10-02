//! Diagnostics and automation, assembled only by composition roots that ask for them (the
//! harness): scenario files, the runner that replays them through the product frame loop, and
//! checkpoint diffs. Never a dependency of a product crate (DECISIONS D10).
#![forbid(unsafe_code)]

mod diff;
mod error;
mod gesture;
mod runner;
mod scenario;

pub use crate::diff::{Diff, diff};
pub use crate::error::ScenarioError;
pub use crate::runner::{Checkpoint, Record, Report, Screenshot, run};
pub use crate::scenario::{Scenario, Step, Tool, is_file_name};
