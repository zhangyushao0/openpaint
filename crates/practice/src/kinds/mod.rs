//! The course's kinds of exercise, a folder each: its generator, answer, check, reading, marks,
//! How (its demonstration), tile and its lessons at each level, and in its `mod.rs` its
//! description (S109, S109b, D111, S122, S123): the one place the rest of the crate and the app
//! and the UI learn a kind from (`Described`, and the original its generator makes, `Original`).
//! A folder that is a skill (`skill.rs`) is one tile on the path. Registering a kind is its
//! variant in `Kind` and its line in `KINDS`; the app and the UI read its topic's facts
//! (`facts.rs`: aligned or compared, its tools and what the eyedropper reads as data in
//! `tools.rs`, what a lesson needs, what the book keeps, how long one takes) and call its
//! original's methods (`Original::compare`, `Original::givens`, `Original::painted`,
//! `Original::answer`) and name its words by its key, never which kind it is.
//!
//! A kind folder may build on another's where the dependency is the course's own: objects are
//! V大's boxes simplified (their answer and check the solids', `solids/`), solids stand in a box
//! whose points are found by the tricks' projection (`geometry/space_projection.rs`).

pub(crate) mod abstract_analysis;
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
mod tools;

pub use crate::marks::compared::DrawnCopy;
pub use facts::{Counted, Facts, Kept, Needs};
pub use kind::Kind;
pub(crate) use kind::{Described, Turn, place};
pub use original::Original;
pub(crate) use original::{Blank, held};
pub use skill::{Skill, SkillLevel};
pub use tools::{Fill, Pen, Picking, ToolSet};

/// Every kind, by its stored code (1 to 21): its line here registers it.
pub(crate) const KINDS: [&dyn Described; 21] = [
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
    &abstract_analysis::Analyses,
];
