//! The course as the path walks it (S122): its units and their skills (`unit.rs`), the learner
//! the records tell of (`learner.rs`: a skill's dots, what opens a unit, the redos waiting) and
//! the lessons offered and laid from them (`schedule.rs`). A skill and its levels are
//! `kinds/skill.rs`; what a lesson of a skill holds at a level is its folder's
//! (`Described::lesson`). This takes the place of the stages' sessions and drills (`sessions/`),
//! which go with S122a.

pub(crate) mod learner;
pub mod schedule;
pub(crate) mod unit;
