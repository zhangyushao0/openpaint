//! Local artwork library: the persistent format (format 2, `records.rs`), OpenPaint files and
//! kept versions, image files in and out, and record spaces whose bytes their owners keep
//! (`spaces.rs`, D112). Implements the storage ports declared in `openpaint-app`.
//!
//! No GPU, no UI (`cargo repo-check`).
//!
//! The modules, each file's `//!` its fields or calls: the library (`library.rs`: `prepare`
//! checks the format and marks an empty database; `convert/` makes a format-1 library and file
//! format 2, S163's build only, D125), `records.rs` (the keys and the
//! head), `strokes.rs` and `shapes.rs` (a stroke), `edits.rs` with `adjusts.rs` and `nested.rs`
//! (an edit record), `catalogue.rs` (entries and thumbnails), `editor.rs` and `editor_library.rs`
//! (the toolbox), `codec.rs` (the byte reader and writer), `delta.rs` (which records a save writes
//! and removes), `read.rs` and `write.rs` (the calls), `batch.rs` (the one atomic batch a call,
//! D77), `spaces.rs` (record spaces, D112); OpenPaint files `artwork_file.rs` (D62) and kept
//! versions `versions.rs`; the display cache `display_cache.rs` (D96); images in and out
//! `colour.rs`, `image_file.rs`, `export_file.rs` (D53, D55).
#![forbid(unsafe_code)]

mod adjusts;
mod artwork_file;
mod batch;
mod catalogue;
mod codec;
mod colour;
mod convert;
mod delta;
mod display_cache;
mod editor;
mod editor_library;
mod edits;
mod export_file;
mod image_file;
mod library;
mod nested;
mod read;
mod records;
mod shapes;
mod spaces;
mod strokes;
mod versions;
mod write;

#[cfg(test)]
mod robustness_tests;
#[cfg(test)]
#[path = "../tests/common/test_dir.rs"]
mod test_dir;

pub use crate::codec::{Reader, Writer};
pub use crate::display_cache::{CacheKey, Cached, DisplayCache};
pub use crate::export_file::{ImageEncoder, encode_image};
pub use crate::image_file::{decode_image, open_picture, open_reference};
pub use crate::library::Library;
pub use crate::records::FORMAT;
