//! A scan's frame cut off (`trim` in a list, S158p): some museums' scans hold a band of the frame
//! or the canvas's edge, which a value masses' answer keeps as a strip and abstract analysis reads
//! as a lightness band. An item's `trim` names the share of the picture cut from each side, top,
//! right, bottom, left, as the list's curator measured it on the contact sheet; it is cut before
//! anything is made of the picture, the packed one and the full one for its crops alike.

use image::RgbaImage;
use image::imageops;
use openpaint_app::ImageFormat;
use openpaint_store::{decode_image, encode_image};

/// The most a side may lose, as a share of the picture's side across it.
pub const MOST: f64 = 0.2;

/// The shares cut from the top, right, bottom and left.
pub type Trim = [f64; 4];

/// Whether `trim` is one a list may give: every share from 0 to `MOST`, at least one above 0.
#[must_use]
pub fn valid(trim: Trim) -> bool {
    trim.iter().all(|share| (0.0..=MOST).contains(share)) && trim.iter().any(|share| *share > 0.0)
}

/// The pixels of `rgba` (`size`, 8-bit RGBA) inside `trim`: their size and pixels.
#[must_use]
pub fn trimmed(size: [u32; 2], rgba: Vec<u8>, trim: Trim) -> ([u32; 2], Vec<u8>) {
    let [width, height] = size;
    let cut = |share: f64, across: u32| (share * f64::from(across)).round() as u32;
    let [top, right, bottom, left] = [
        cut(trim[0], height),
        cut(trim[1], width),
        cut(trim[2], height),
        cut(trim[3], width),
    ];
    let kept = [
        width.saturating_sub(left + right).max(1),
        height.saturating_sub(top + bottom).max(1),
    ];
    let Some(picture) = RgbaImage::from_raw(width, height, rgba) else {
        return (size, Vec::new());
    };
    let inside = imageops::crop_imm(&picture, left, top, kept[0], kept[1]).to_image();
    (kept, inside.into_raw())
}

/// The picture file `original` inside `trim`, as a PNG (lossless, so the pack's own reduction
/// and encoding follow as for any download).
///
/// # Errors
/// A description when `original` does not decode or the trimmed picture does not encode.
pub fn trimmed_file(original: &[u8], trim: Trim) -> Result<Vec<u8>, String> {
    let (size, rgba) = decode_image(original).map_err(|error| error.to_string())?;
    let (kept, inside) = trimmed(size, rgba, trim);
    encode_image(ImageFormat::Png, kept, &inside).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trim_cuts_each_side_by_its_share() {
        let size = [10, 20];
        let rgba: Vec<u8> = (0..10 * 20).flat_map(|at| [at as u8, 0, 0, 255]).collect();
        let (kept, inside) = trimmed(size, rgba, [0.1, 0.2, 0.05, 0.1]);
        assert_eq!(kept, [7, 17]);
        // The first pixel kept is the one at column 1, row 2.
        assert_eq!(inside[0], 21);
    }

    #[test]
    fn only_shares_up_to_a_fifth_and_one_above_zero_are_valid() {
        assert!(valid([0.0, 0.0, 0.05, 0.0]));
        assert!(!valid([0.0; 4]));
        assert!(!valid([0.3, 0.0, 0.0, 0.0]));
        assert!(!valid([-0.01, 0.0, 0.05, 0.0]));
    }
}
