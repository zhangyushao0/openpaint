//! A scenario's `lesson` step (S122): a lesson of the path started from the home as a tap there
//! would start it (`Runtime::start_lesson`), for walks that begin on a lesson without tapping
//! their way to it. Its shapes:
//!
//! ```json
//! { "lesson": { "skill": "line", "level": 1 } }
//! { "lesson": { "skill": "points-in-space" } }
//! { "lesson": "first" }
//! { "lesson": "next" }
//! ```
//!
//! A skill goes by its key (`Skill::key`: `line`, `colour`, `free-boxes`…), its level 1 to 3
//! (入门, 熟练, 稳; without one the learner's); `first` the first
//! lesson whatever was practised; `next` what the home's main control does: Start's lesson, or
//! Continue's (the day's unfinished lesson opened) while one is under way
//! (`Runtime::start_lesson` with `Start::Next`).

use serde::Deserialize;

use openpaint_runtime::{RuntimeError, Start, skill_start};

/// The lesson a `lesson` step starts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Lesson {
    /// A lesson of the skill keyed `skill`, at `level` (1 to 3) or the learner's.
    Skill {
        skill: String,
        #[serde(default)]
        level: Option<u8>,
    },
    /// `first` or `next`.
    Named(Named),
}

/// A lesson named by a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Named {
    First,
    Next,
}

impl Lesson {
    /// What the runtime starts for it.
    ///
    /// # Errors
    /// `NotNow` for a key no skill has, or a level past 3.
    pub fn start(&self) -> Result<Start, RuntimeError> {
        match self {
            Self::Skill { skill, level } => skill_start(skill, *level),
            Self::Named(Named::First) => Ok(Start::First),
            Self::Named(Named::Next) => Ok(Start::Next),
        }
    }
}
