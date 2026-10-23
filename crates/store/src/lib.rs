//! Local artwork library: the persistent format (format 1, `records.rs`), OpenPaint files and
//! kept versions, image files in and out, practice packs kept beside the library. Implements the
//! storage ports declared in `openpaint-app`.
//!
//! No GPU, no UI (`cargo repo-check`).
#![forbid(unsafe_code)]

mod adjusts;
mod artwork_file;
mod batch;
mod catalogue;
mod codec;
mod colour;
mod delta;
mod display_cache;
mod editor;
mod editor_library;
mod edits;
mod export_file;
mod image_file;
mod library;
mod nested;
mod pack;
mod packs;
mod practice;
mod read;
mod records;
mod shapes;
mod strokes;
mod versions;
mod write;

#[cfg(test)]
mod robustness_tests;

pub use crate::display_cache::{CacheKey, Cached, DisplayCache};
pub use crate::export_file::{ImageEncoder, encode_image};
pub use crate::image_file::{decode_image, open_picture, open_reference};
pub use crate::library::Library;
pub use crate::pack::{PACK_FORMAT, encode_pack, open_pack};
pub use crate::packs::Packs;
pub use crate::records::FORMAT;
