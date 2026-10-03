//! Local artwork library: persistent format, schema migrations, assets. Implements the
//! storage ports declared in `openpaint-app`.
//!
//! No GPU, no UI (`cargo repo-check`).
#![forbid(unsafe_code)]

mod catalogue;
mod codec;
mod delta;
mod library;
mod migrate;
mod read;
mod records;
mod write;

pub use crate::library::Library;
pub use crate::records::SCHEMA;
