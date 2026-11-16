//! The course as the path walks it (S122): its units and their skills (`unit.rs`), the learner
//! the records tell of (`learner.rs`: a skill's dots, what opens a unit, the redos waiting), the
//! lessons offered and laid from them (`schedule.rs`), how a lesson picks its pictures and turns
//! its variants (`picker.rs`) and a unit's re-tests at its start and its end (`retest.rs`, S147).
//! A skill and its levels are `kinds/skill.rs`; what a lesson of a skill holds at a level is its
//! folder's (`Described::lesson`).

pub(crate) mod learner;
pub(crate) mod picker;
pub mod retest;
pub mod schedule;
pub(crate) mod unit;
