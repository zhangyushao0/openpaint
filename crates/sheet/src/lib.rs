//! Contact sheets for the development tools (S111): what the practice-pack builder
//! (`tools/packs`) and the harness's `key-shifts` both draw their sheets with, so neither
//! depends on the other. A sheet is a grid of `CELL`-pixel cells on grey; a picture fitted into a
//! cell, filled rectangles, lines `width` pixels wide and a number in a 3 x 5 pixel font at a
//! cell's corner; written as an 8-bit RGBA PNG. Never a dependency of a product crate.
#![forbid(unsafe_code)]

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};

/// A cell's side, in sheet pixels.
pub const CELL: u32 = 480;
/// A digit's pixel, in sheet pixels.
const DOT: u32 = 4;

/// The digits 0 to 9, 3 x 5 dots each, rows top first, three bits a row (left the highest).
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

/// A picture of `size` as large as a cell shows it.
#[must_use]
pub fn fitted([width, height]: [u32; 2]) -> [u32; 2] {
    let scale = (f64::from(CELL) / f64::from(width.max(height).max(1))).min(1.0);
    [width, height].map(|length| ((f64::from(length) * scale).round() as u32).max(1))
}

/// The picture `rgba` of `size` fitted inside a cell, resized by `filter`; `None` when the
/// pixels are not a picture of `size`.
#[must_use]
pub fn thumbnail(size: [u32; 2], rgba: &[u8], filter: FilterType) -> Option<RgbaImage> {
    let image = RgbaImage::from_raw(size[0], size[1], rgba.to_vec())?;
    let [width, height] = fitted(size);
    Some(imageops::resize(&image, width, height, filter))
}

/// `lines` (points `place` puts on the sheet) in `colour`, `width` sheet pixels wide.
pub fn draw_in(
    sheet: &mut RgbaImage,
    lines: &[Vec<[f64; 2]>],
    place: &dyn Fn([f64; 2]) -> [f64; 2],
    (colour, width): (Rgba<u8>, f64),
) {
    for line in lines {
        for pair in line.windows(2) {
            let [a, b] = [place(pair[0]), place(pair[1])];
            let steps = ((b[0] - a[0]).hypot(b[1] - a[1]) * 2.0).ceil().max(1.0) as usize;
            for step in 0..=steps {
                let t = step as f64 / steps as f64;
                let [cx, cy] = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
                let corner = |v: f64| (v - width / 2.0).round().max(0.0) as u32;
                let side = width.round().max(1.0) as u32;
                fill(sheet, [corner(cx), corner(cy)], [side, side], colour);
            }
        }
    }
}

/// The rectangle at `[x, y]` of `[width, height]` in `colour`, cut at the sheet's edges.
pub fn fill(sheet: &mut RgbaImage, [x, y]: [u32; 2], [width, height]: [u32; 2], colour: Rgba<u8>) {
    for row in y..(y + height).min(sheet.height()) {
        for column in x..(x + width).min(sheet.width()) {
            sheet.put_pixel(column, row, colour);
        }
    }
}

/// `value` in black on a white tab at the cell's top left corner `[x, y]`.
pub fn number(sheet: &mut RgbaImage, [x, y]: [u32; 2], value: usize) {
    let digits: Vec<usize> = value
        .to_string()
        .bytes()
        .map(|b| usize::from(b - b'0'))
        .collect();
    let width = digits.len() as u32 * 4 * DOT + DOT;
    fill(sheet, [x, y], [width, 7 * DOT], Rgba([255, 255, 255, 255]));
    for (place, digit) in digits.into_iter().enumerate() {
        for (row, bits) in DIGITS[digit].iter().enumerate() {
            for column in 0..3 {
                if bits & (4 >> column) != 0 {
                    let left = x + DOT + (place as u32 * 4 + column) * DOT;
                    let top = y + DOT + row as u32 * DOT;
                    fill(sheet, [left, top], [DOT, DOT], Rgba([0, 0, 0, 255]));
                }
            }
        }
    }
}

/// Writes 8-bit RGBA pixels of `size` as a PNG at `path`; the directory must exist.
///
/// # Errors
/// The encoder's, a file that cannot be made among them.
pub fn write_rgba(path: &Path, size: [u32; 2], pixels: &[u8]) -> Result<(), png::EncodingError> {
    let [width, height] = size;
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)?;
    writer.finish()?;
    Ok(())
}
