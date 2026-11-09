//! Practice in the library and beside it (S110, D112): its records' bytes in the library's
//! record space `PRACTICE` (`records.rs`), read, started, saved and removed through the store's
//! port (`stored.rs`); and the practice packs kept beside the library, their file format and
//! their pictures (`pack*.rs`); and what each exercise of a sheet printed, as bytes (`givens/`).
//!
//! No GPU, no UI (`cargo repo-check`).
#![forbid(unsafe_code)]

mod givens;
mod pack;
mod pack_picture;
mod packs;
mod records;
mod stored;

pub use crate::givens::{
    GIVENS_FORMAT, Given, Givens, decode_givens, decode_marks, decode_painted, encode_givens,
    encode_marks, encode_painted,
};
pub use crate::pack::{PACK_FORMAT, encode_pack, open_pack};
pub use crate::pack_picture::{PackPicture, pack_picture, shown_picture, thumbnail_picture};
pub use crate::packs::Packs;
pub use crate::records::PRACTICE;
pub use crate::stored::{remove, save, sessions, start};
