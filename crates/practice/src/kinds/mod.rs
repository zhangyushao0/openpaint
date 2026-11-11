//! The course's kinds of exercise, a folder each: its generator, answer, check, reading, marks,
//! How (its demonstration) and tile, and in its `mod.rs` its description (S109, S109b, D111): the
//! one place the rest of the crate and the app and the UI learn a kind from (`Described`).
//! Registering a kind is its variant in `Kind` (and in `Original` when it brings a new answer) and
//! its line in `KINDS`; the app and the UI ask its families' facts (`facts.rs`: work, tools, what
//! the eyedropper reads, what a drill needs, what the book keeps, the summary's rows) and its
//! description's methods (`Exercise::compare`, `Exercise::painted`, `Family::tile`,
//! `Family::answer`) and name its words by its key, never which kind it is.
//!
//! A kind folder may build on another's where the dependency is the course's own: objects are
//! V大's boxes simplified (their answer and check the solids', `solids/`), solids stand in a box
//! whose points are found by the tricks' projection (`geometry/space_projection.rs`).

pub(crate) mod boxes;
pub(crate) mod colours;
pub(crate) mod combinations;
pub(crate) mod combined_lines;
pub(crate) mod divisions;
pub(crate) mod ellipses;
pub(crate) mod estimates;
pub(crate) mod free;
pub(crate) mod free_boxes;
pub(crate) mod generated_combinations;
pub(crate) mod key_shifts;
pub(crate) mod lines;
pub(crate) mod objects;
pub(crate) mod points_in_space;
pub(crate) mod scales;
pub(crate) mod shapes;
pub(crate) mod solids;
pub(crate) mod study;
pub(crate) mod value_masses;
pub(crate) mod whole_copies;

mod facts;
mod kind;
mod original;
mod skill;

pub use crate::marks::compared::DrawnCopy;
pub use facts::{Facts, Kept, Needs, Picks, Tools, Work};
pub use kind::Kind;
pub(crate) use kind::{Described, Turn, place};
pub use original::Original;
pub use skill::{Skill, SkillLevel};

/// Every kind, by its stored code (1 to 20): its line here registers it.
pub(crate) const KINDS: [&dyn Described; 20] = [
    &lines::Lines,
    &colours::Colours,
    &combined_lines::CombinedLines,
    &shapes::Shapes,
    &generated_combinations::Generated,
    &scales::Scales,
    &study::Study,
    &boxes::Boxes,
    &free_boxes::FreeBoxes,
    &ellipses::Ellipses,
    &estimates::Estimates,
    &points_in_space::PointsInSpace,
    &solids::Solids,
    &objects::Objects,
    &divisions::Divisions,
    &whole_copies::WholeCopies,
    &value_masses::ValueMasses,
    &combinations::Crops,
    &key_shifts::KeyShifts,
    &free::Free,
];
