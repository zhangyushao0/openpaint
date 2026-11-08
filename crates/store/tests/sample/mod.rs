//! The format-1 sample (`tests/samples/format-1.bin`): the sample artwork holding every kind of
//! record, its title, thumbnail and toolbox, and the record dump format.

use std::path::PathBuf;

use openpaint_app::{Thumbnail, Title};

mod artwork;
mod dump;
mod editor;
mod records;
pub use dump::{decode, encode};
pub use editor::sample_editor;
pub use records::parts;

pub type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

/// The sample's file, found as well from a crate beside the store whose tests take this module
/// by path.
pub fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../store/tests/samples/format-1.bin")
}

pub fn sample_title() -> Title {
    Title::new("Hafen ✒ 墨").expect("title")
}

/// Two pixels: opaque red, half-transparent grey.
pub fn sample_thumbnail() -> Thumbnail {
    Thumbnail::new([2, 1], vec![255, 0, 0, 255, 128, 128, 128, 128]).expect("thumbnail")
}
