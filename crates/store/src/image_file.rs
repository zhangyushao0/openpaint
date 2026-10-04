//! Picture files read: PNG or JPEG files as reference images or pictures to insert. The `image`
//! crate does the decoding (DECISIONS D14); a file's embedded ICC profile is honoured, so colours
//! come in as sRGB (`colour`, S27, D53). Writing is `export_file.rs`.

use std::io::Cursor;

use image::imageops::{self, FilterType as Filter};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};
use openpaint_app::{ImageError, MAX_IMAGE_PIXELS, ReferenceImage, picture_place};
use openpaint_core::{CanvasSize, Patch};

use crate::colour::{Interpretation, into_srgb};

/// The picture in a PNG or JPEG file as 8-bit sRGB RGBA at its full size, turned upright as its
/// EXIF orientation says (camera pictures). An embedded RGB ICC profile (a phone's Display P3)
/// converts the colours into sRGB; an untagged file, or one whose profile is sRGB, unusable or
/// not RGB, keeps its values (`colour::into_srgb`).
///
/// # Errors
/// `Unsupported` for any other file, `TooLarge` above `MAX_IMAGE_PIXELS`, `Damaged` when the
/// file breaks its format.
pub fn decode_image(bytes: &[u8]) -> Result<([u32; 2], Vec<u8>), ImageError> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| ImageError::Damaged)?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err(ImageError::Unsupported);
    }
    let mut decoder = reader.into_decoder().map_err(decoding)?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(ImageError::TooLarge);
    }
    let orientation = decoder.orientation().map_err(decoding)?;
    let profile = decoder.icc_profile().map_err(decoding)?;
    let mut picture = DynamicImage::from_decoder(decoder).map_err(decoding)?;
    picture.apply_orientation(orientation);
    let mut rgba = picture.into_rgba8();
    match into_srgb(profile.as_deref(), &mut rgba) {
        Interpretation::Converted => log::info!("picture converted from its ICC profile to sRGB"),
        Interpretation::Ignored => {
            log::warn!("picture's ICC profile unreadable or not RGB; its values taken as sRGB");
        }
        Interpretation::Untagged | Interpretation::Srgb => {}
    }
    Ok(([rgba.width(), rgba.height()], rgba.into_raw()))
}

/// A PNG or JPEG file as a reference image: decoded, on white, scaled down to fit
/// `MAX_REFERENCE_SIDE`.
///
/// # Errors
/// As `decode_image`.
pub fn open_reference(bytes: &[u8]) -> Result<ReferenceImage, ImageError> {
    let (size, pixels) = decode_image(bytes)?;
    ReferenceImage::fitted(size, &pixels).ok_or(ImageError::Damaged)
}

/// A PNG or JPEG file as a picture to insert into a canvas of `canvas` (S26): decoded, turned
/// upright, premultiplied, reduced once by a triangle filter when it is larger than the canvas
/// (on premultiplied pixels, so a transparent pixel's colour never bleeds into its neighbours)
/// and centred (`picture_place`). Colours come in as sRGB (`decode_image`).
///
/// # Errors
/// As `decode_image`.
pub fn open_picture(bytes: &[u8], canvas: CanvasSize) -> Result<Patch, ImageError> {
    let (size, mut pixels) = decode_image(bytes)?;
    let (origin, fitted) = picture_place(canvas, size).ok_or(ImageError::Damaged)?;
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    if fitted != size {
        let image = RgbaImage::from_raw(size[0], size[1], pixels).ok_or(ImageError::Damaged)?;
        let reduced = imageops::resize(&image, fitted[0], fitted[1], Filter::Triangle);
        pixels = reduced.into_raw();
        // Rounding each channel alone may lift a colour a step above its alpha.
        for pixel in pixels.as_chunks_mut::<4>().0 {
            let alpha = pixel[3];
            for channel in &mut pixel[..3] {
                *channel = (*channel).min(alpha);
            }
        }
    }
    Patch::new(canvas, origin, fitted, pixels).map_err(|_| ImageError::Damaged)
}

fn decoding(error: image::ImageError) -> ImageError {
    match error {
        image::ImageError::Limits(_) => ImageError::TooLarge,
        image::ImageError::Unsupported(_) => ImageError::Unsupported,
        _ => ImageError::Damaged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export_file::encode_image;
    use openpaint_app::{ExportError, ImageFormat};

    /// 3 x 2 pixels of distinct colours, one half transparent.
    fn pixels() -> Vec<u8> {
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, //
            10, 20, 30, 255, 200, 100, 50, 128, 255, 255, 255, 255,
        ]
    }

    #[test]
    fn png_keeps_every_byte() {
        let bytes = encode_image(ImageFormat::Png, [3, 2], &pixels()).expect("encode");
        assert_eq!(&bytes[..4], b"\x89PNG");
        assert_eq!(decode_image(&bytes), Ok(([3, 2], pixels())));
    }

    #[test]
    fn jpeg_is_close_and_opaque() {
        let flat: Vec<u8> = (0..16 * 16).flat_map(|_| [40, 120, 200, 255]).collect();
        let bytes = encode_image(ImageFormat::Jpeg, [16, 16], &flat).expect("encode");
        assert_eq!(&bytes[..2], b"\xff\xd8");
        let (size, decoded) = decode_image(&bytes).expect("decode");
        assert_eq!(size, [16, 16]);
        for (got, want) in decoded.iter().zip(&flat) {
            assert!(got.abs_diff(*want) <= 3, "{got} against {want}");
        }
    }

    /// `jpeg` with an EXIF segment saying "rotate 90 degrees clockwise to view" (orientation 6),
    /// as a camera writes it.
    fn rotated(jpeg: &[u8]) -> Vec<u8> {
        let mut tiff = b"II*\0\x08\0\0\0\x01\0".to_vec();
        tiff.extend([0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0]);
        let mut segment = vec![0xff, 0xe1, 0, (2 + 6 + tiff.len()) as u8];
        segment.extend(b"Exif\0\0");
        segment.extend(tiff);
        [&jpeg[..2], &segment, &jpeg[2..]].concat()
    }

    #[test]
    fn camera_pictures_are_turned_upright() {
        // 16 x 8: red left half, blue right half.
        let pixels: Vec<u8> = (0..16 * 8)
            .flat_map(|i| {
                if i % 16 < 8 {
                    [220, 30, 30, 255]
                } else {
                    [30, 30, 220, 255]
                }
            })
            .collect();
        let jpeg = encode_image(ImageFormat::Jpeg, [16, 8], &pixels).expect("encode");
        let (size, upright) = decode_image(&rotated(&jpeg)).expect("decode");
        assert_eq!(size, [8, 16], "turned a quarter clockwise");
        let red = |at: usize| upright[at * 4] > 150 && upright[at * 4 + 2] < 100;
        assert!(red(2 * 8 + 4), "the left half is on top");
        assert!(!red(13 * 8 + 4), "the right half is below");
    }

    fn canvas(width: u32, height: u32) -> CanvasSize {
        CanvasSize::new(width, height).expect("canvas")
    }

    #[test]
    fn a_small_picture_is_premultiplied_pixel_for_pixel_and_centred() {
        let bytes = encode_image(ImageFormat::Png, [3, 2], &pixels()).expect("encode");
        let patch = open_picture(&bytes, canvas(9, 6)).expect("picture");
        assert_eq!((patch.origin(), patch.size()), ([3, 2], [3, 2]));
        // The half transparent pixel (200, 100, 50, 128) is premultiplied; the rest are opaque.
        assert_eq!(&patch.pixels()[16..20], [100, 50, 25, 128]);
        assert_eq!(&patch.pixels()[..4], [255, 0, 0, 255]);
        assert!(
            open_picture(b"no picture", canvas(9, 6)).is_err(),
            "refused as before"
        );
    }

    #[test]
    fn a_large_camera_picture_is_turned_upright_and_fitted_inside() {
        // 16 x 8, red left half and blue right half, the camera saying "turn a quarter": upright
        // it is 8 x 16 with red on top, fitted into 10 x 4 as 2 x 4.
        let pixels: Vec<u8> = (0..16 * 8)
            .flat_map(|i| {
                if i % 16 < 8 {
                    [220, 30, 30, 255]
                } else {
                    [30, 30, 220, 255]
                }
            })
            .collect();
        let jpeg = encode_image(ImageFormat::Jpeg, [16, 8], &pixels).expect("encode");
        let patch = open_picture(&rotated(&jpeg), canvas(10, 4)).expect("picture");
        assert_eq!((patch.origin(), patch.size()), ([4, 0], [2, 4]));
        let red = |row: usize| patch.pixels()[row * 8] > 150 && patch.pixels()[row * 8 + 2] < 100;
        assert!(red(0) && !red(3), "red on top, blue below");
        assert!(
            patch
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[3] == 255)
        );
    }

    #[test]
    fn reducing_never_bleeds_a_transparent_colour_in() {
        // 4 x 1: opaque white, then transparent pixels whose colour is pure red.
        let pixels = [255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0];
        let bytes = encode_image(ImageFormat::Png, [4, 1], &pixels).expect("encode");
        let patch = open_picture(&bytes, canvas(2, 2)).expect("picture");
        assert_eq!(patch.size(), [2, 1]);
        for pixel in patch.pixels().as_chunks::<4>().0 {
            let [r, g, b, a] = *pixel;
            assert!(r <= a && g <= a && b <= a, "premultiplied: {pixel:?}");
            assert_eq!(r, g, "no red in the white: {pixel:?}");
        }
    }

    #[test]
    fn other_files_are_refused_by_kind() {
        assert_eq!(
            decode_image(b"hello, not a picture"),
            Err(ImageError::Unsupported)
        );
        let bytes = encode_image(ImageFormat::Png, [3, 2], &pixels()).expect("encode");
        assert_eq!(decode_image(&bytes[..40]), Err(ImageError::Damaged));
        assert_eq!(
            encode_image(ImageFormat::Png, [3, 3], &pixels()),
            Err(ExportError::Encode)
        );
        let reference = open_reference(&bytes).expect("reference");
        assert_eq!(reference.size(), [3, 2]);
    }
}
