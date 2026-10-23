//! A picture as a practice pack holds it (D101), the same for the official packs the harness
//! builds and the user's own pictures the personal pack takes: decoded into sRGB and turned
//! upright (`decode_image`), its long side reduced to `MAX_PICTURE_SIDE` when longer (Lanczos-3
//! over premultiplied pixels, so no transparent pixel's colour bleeds in), then a PNG when a pixel
//! is transparent, else a JPEG (the export encoders: sRGB declared, no other metadata). Decoding,
//! reducing and encoding are exact, so the same file gives the same bytes.

use image::RgbaImage;
use image::imageops::{self, FilterType};
use openpaint_app::{ImageError, ImageFormat, PictureImage, SHOWN_SIDE};
use openpaint_practice::MAX_PICTURE_SIDE;

use crate::export_file::encode_image;
use crate::image_file::decode_image;

/// A picture ready for a pack: its size, its pixels (8-bit sRGB, straight alpha) and its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackPicture {
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
    pub file: Vec<u8>,
}

/// The PNG or JPEG file `original` as a pack holds it.
///
/// # Errors
/// As `decode_image`; `Damaged` when the reduced picture does not encode.
pub fn pack_picture(original: &[u8]) -> Result<PackPicture, ImageError> {
    let ([width, height], rgba) = decode_image(original)?;
    let mut picture = RgbaImage::from_raw(width, height, rgba).ok_or(ImageError::Damaged)?;
    let long = width.max(height);
    if long > MAX_PICTURE_SIDE {
        let scale = f64::from(MAX_PICTURE_SIDE) / f64::from(long);
        let side = |length: u32| ((f64::from(length) * scale).round() as u32).clamp(1, long);
        picture = reduced(&picture, [side(width), side(height)]);
    }
    let opaque = picture.pixels().all(|pixel| pixel[3] == 255);
    let format = if opaque {
        ImageFormat::Jpeg
    } else {
        ImageFormat::Png
    };
    let size = [picture.width(), picture.height()];
    let file = encode_image(format, size, picture.as_raw()).map_err(|_| ImageError::Damaged)?;
    Ok(PackPicture {
        size,
        rgba: picture.into_raw(),
        file,
    })
}

/// A pack's picture file as the screen shows it: decoded, its long side reduced to `SHOWN_SIDE`
/// when longer (as a pack's are).
///
/// # Errors
/// As `decode_image`.
pub fn shown_picture(file: &[u8]) -> Result<PictureImage, ImageError> {
    side_at_most(file, SHOWN_SIDE, FilterType::Lanczos3)
}

/// A pack's picture file as a list shows it small (the practice options, S83): decoded, its
/// long side reduced to `side` with a triangle filter, quicker than `shown_picture`'s.
///
/// # Errors
/// As `decode_image`.
pub fn thumbnail_picture(file: &[u8], side: u32) -> Result<PictureImage, ImageError> {
    side_at_most(file, side, FilterType::Triangle)
}

/// The picture file `file` decoded, its long side reduced to `most` by `filter` when longer.
fn side_at_most(file: &[u8], most: u32, filter: FilterType) -> Result<PictureImage, ImageError> {
    let ([width, height], rgba) = decode_image(file)?;
    let picture = RgbaImage::from_raw(width, height, rgba).ok_or(ImageError::Damaged)?;
    let long = width.max(height);
    let picture = if long > most {
        let scale = f64::from(most) / f64::from(long);
        let side = |length: u32| ((f64::from(length) * scale).round() as u32).clamp(1, long);
        resized(&picture, [side(width), side(height)], filter)
    } else {
        picture
    };
    Ok(PictureImage {
        size: [picture.width(), picture.height()],
        pixels: picture.into_raw(),
    })
}

/// `picture` resized to `size` through premultiplied pixels with Lanczos-3.
fn reduced(picture: &RgbaImage, size: [u32; 2]) -> RgbaImage {
    resized(picture, size, FilterType::Lanczos3)
}

/// `picture` resized to `size` through premultiplied pixels by `filter`.
fn resized(picture: &RgbaImage, size: [u32; 2], filter: FilterType) -> RgbaImage {
    let mut premultiplied = picture.clone();
    for pixel in premultiplied.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    let mut out = imageops::resize(&premultiplied, size[0], size[1], filter);
    for pixel in out.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = match alpha {
                0 => 0,
                255 => *channel,
                _ => ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8,
            };
        }
    }
    out
}
