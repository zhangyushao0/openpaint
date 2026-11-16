//! Placement from a floor plan (S153): the plan keeps its rules and the eye's sight its, seed after
//! seed; the answer's corners lie on V大's construction found from the givens alone; Compare lays
//! the answer for the eye and never reads the copy; the How's example lands on its answer.

mod compare;
mod exact;
mod how;
mod rules;

use openpaint_practice::{Placement, PlacementTask, derived};

/// The seed the tests lay placements from.
const SEED: u64 = 0x5EED_0153;
/// Placements of each variant the tests lay.
const SEEDS: u64 = 24;

/// Every variant's first `SEEDS` placements of `SEED`'s sequence.
fn laid() -> Vec<PlacementTask> {
    (Placement::ALL.iter())
        .flat_map(|placement| {
            (0..SEEDS).map(|k| PlacementTask::generate(*placement, derived(SEED, k)))
        })
        .collect()
}
