//! Local artwork library: the persistent format (format 1, `records.rs`), OpenPaint files and
//! kept versions, image files in and out, and record spaces whose bytes and upgrades their owners
//! keep (`spaces.rs`, D112). Implements the storage ports declared in `openpaint-app`.
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
pub use crate::spaces::{Upgrade, Upgrading};
