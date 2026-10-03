//! Pure pixel geometry on the CPU: the connected region a fill covers (ColorDrop) and the
//! pixels a transform lifts.
//!
//! Depends on `openpaint-core` only; no GPU, no threads, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

mod fill;
mod lift;

pub use fill::{FillError, connected_region};
pub use lift::lift;
