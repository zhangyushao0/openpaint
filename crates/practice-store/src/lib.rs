//! Practice in the library and beside it (S110, D112): its sheets' records in the library's
//! record space `PRACTICE` and their lessons' givens in `GIVENS` (`records.rs`), read, started,
//! saved and removed through the store's port (`stored.rs`); the practice packs kept beside the
//! library, their file format and their pictures (`pack*.rs`); what each exercise of a sheet
//! printed, as bytes (`givens/`); the Pomodoro's settings and blocks in `TIMER` (S124,
//! `blocks.rs`); the blind pairs answered in `PAIRS` (S147, `pairs.rs`); and the practice-only
//! clear of them all (S152, `clear.rs`).
//!
//! No GPU, no UI (`cargo repo-check`).
#![forbid(unsafe_code)]

mod blocks;
mod clear;
mod givens;
mod pack;
mod pack_picture;
mod packs;
mod pairs;
mod records;
mod stored;

pub use crate::blocks::{TIMER, blocks, forget_blocks, save_timer};
pub use crate::clear::{Cleared, clear};
pub use crate::givens::{
    GIVENS_FORMAT, decode_givens, decode_marks, decode_painted, encode_givens, encode_marks,
    encode_painted,
};
pub use crate::pack::{PACK_FORMAT, encode_pack, open_pack};
pub use crate::pack_picture::{PackPicture, pack_picture, shown_picture, thumbnail_picture};
pub use crate::packs::Packs;
pub use crate::pairs::{PAIRS, forget_pairs, pairs, put_pair};
pub use crate::records::{GIVENS, PRACTICE};
pub use crate::stored::{append, printed, remove, save, sheets, start, trim};
