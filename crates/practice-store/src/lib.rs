//! Practice in the library and beside it (S110, D112): its records' bytes in the library's
//! record space `PRACTICE` (`records.rs`) and their upgrade (`upgrade.rs`, `frames.rs`),
//! registered by `library_at`, the one way the hosts and tools open a library; the records read,
//! started, saved and removed through the store's port (`stored.rs`); and the practice packs
//! kept beside the library, their file format and its upgrade, and their pictures (`pack*.rs`).
//!
//! No GPU, no UI (`cargo repo-check`).
#![forbid(unsafe_code)]

mod frames;
mod pack;
mod pack_picture;
mod pack_upgrade;
mod packs;
mod records;
mod stored;
mod upgrade;

pub use crate::pack::{PACK_FORMAT, encode_pack, open_pack};
pub use crate::pack_picture::{PackPicture, pack_picture, shown_picture, thumbnail_picture};
pub use crate::packs::Packs;
pub use crate::records::PRACTICE;
pub use crate::stored::{remove, save, sessions, start};
pub use crate::upgrade::library_at;
