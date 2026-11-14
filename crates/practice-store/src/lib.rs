//! Practice in the library and beside it (S110, D112): its sheets' records in the library's
//! record space `PRACTICE` and their lessons' givens in `GIVENS` (`records.rs`), read, started,
//! saved and removed through the store's port (`stored.rs`); the practice packs kept beside the
//! library, their file format and their pictures (`pack*.rs`); what each exercise of a sheet
//! printed, as bytes (`givens/`); and the Pomodoro's settings and blocks in `TIMER` (S124,
//! `blocks.rs`).
//!
//! No GPU, no UI (`cargo repo-check`).
#![forbid(unsafe_code)]

mod blocks;
mod givens;
mod pack;
mod pack_picture;
mod packs;
mod records;
mod stored;

pub use crate::blocks::{TIMER, blocks, save_timer};
pub use crate::givens::{
    GIVENS_FORMAT, decode_givens, decode_marks, decode_painted, encode_givens, encode_marks,
    encode_painted,
};
pub use crate::pack::{PACK_FORMAT, encode_pack, open_pack};
pub use crate::pack_picture::{PackPicture, pack_picture, shown_picture, thumbnail_picture};
pub use crate::packs::Packs;
pub use crate::records::{GIVENS, PRACTICE};
pub use crate::stored::{append, printed, remove, save, sheets, start, trim};
