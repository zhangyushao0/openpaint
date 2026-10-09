//! Local artwork library: persistent format, schema migrations, assets. Implements the
//! storage ports declared in `openpaint-app`.
//!
//! No GPU, no UI (`cargo repo-check`).
#![forbid(unsafe_code)]

mod adjusts;
mod artwork_file;
mod catalogue;
mod codec;
mod colour;
mod delta;
mod editor;
mod export_file;
mod fills;
mod image_file;
mod library;
mod migrate;
mod migrate_10;
mod migrate_11;
mod migrate_12;
mod migrate_13;
mod migrate_14;
mod migrate_15;
mod migrate_16;
mod migrate_17;
mod migrate_18;
mod migrate_6;
mod migrate_7;
mod migrate_8;
mod migrate_9;
mod nested;
mod read;
mod records;
mod steps;
mod strokes;
mod versions;
mod write;

pub use crate::export_file::{ImageEncoder, encode_image};
pub use crate::image_file::{decode_image, open_picture, open_reference};
pub use crate::library::Library;
pub use crate::records::SCHEMA;
