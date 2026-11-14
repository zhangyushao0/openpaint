//! Diagnostics and automation, assembled only by composition roots that ask for them (the
//! harness): scenario files, the runner that replays them through the product frame loop, and
//! checkpoint diffs. Never a dependency of a product crate (DECISIONS D10).
//!
//! A scenario file (`scenario.rs`; D19, D48) holds a walk-through (text), the screen size and
//! density, the host's `system_language` (English unless `"chinese"`, D100), `start` (a blank
//! canvas, or `"home"` the first launch, D116) and the steps that replay it: `lesson`
//! (`lesson.rs`: `{ "skill": "line", "level": 1 }`, `"first"`, `"next"` the home's main control),
//! `host` (`host.rs`: the chimes played and the screen kept on), `tap`, `drag`, `stroke` (with
//! `lift`, `palm`, `sparse`, `pen` buttons or the eraser end, `hold_ms`, `tilt`, `azimuth`),
//! `press` (`[x, y, ms]` keys, `before_lift`, `interrupted` by `detach` or `leave`), `touch` (1 to
//! 5 fingers), `hover`, `type` (a newline presses Enter), `back`, `wait`, `later` (minutes on at
//! once, the app untouched or `hidden`; the held clock, a Pomodoro's too), `restart`,
//! `storage_fails` / `storage_works`, `system` (a picker's answer: a fixture picture, the test pack
//! or test frames of `test_pack.rs`, the test compositions of `test_compositions.rs`, the test
//! crops of known areas of `test_crops.rs`), `lose_device`, `checkpoint` (a canvas hash), `view`,
//! `exported`, `preview`, `expect` (controls that must and must not show) and `screenshot`. A spot
//! is a control's name (`name #n` when shared), `{ on, fraction, offset }`, or window pixels on the
//! canvas only (`target.rs`); pixels on a control fail the run with the name to write. The runner
//! (`runner.rs`) holds the clock and opens one device with an 8-bit screen; `Watched` is the
//! switchable store wrapper `storage_fails` turns.
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
