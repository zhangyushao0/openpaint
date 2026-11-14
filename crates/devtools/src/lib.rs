//! Diagnostics and automation, assembled only by composition roots that ask for them (the
//! harness): scenario files, the runner that replays them through the product frame loop, and
//! checkpoint diffs. Never a dependency of a product crate (DECISIONS D10).
#![forbid(unsafe_code)]

mod diff;
mod error;
mod gesture;
mod host;
mod lesson;
mod pen;
mod report;
mod runner;
mod scenario;
mod system;
mod target;
mod test_compositions;
mod test_crops;
mod test_pack;
mod watched;

pub use crate::diff::{Diff, diff};
pub use crate::error::ScenarioError;
pub use crate::host::HostExpect;
pub use crate::pen::Pen;
pub use crate::report::{Checkpoint, Record, Report, Screenshot, ViewCheck};
pub use crate::runner::run;
pub use crate::scenario::{
    Interruption, Lands, Lesson, Palm, Scenario, StartOn, Step, SystemAnswer, Tool, is_file_name,
};
pub use crate::system::{Export, TEST_BANDS};
pub use crate::target::{Key, Place, Spot};
pub use crate::watched::{Watched, Wrote};
