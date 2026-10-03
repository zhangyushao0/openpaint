//! Picture files: an artwork's pixels written as PNG or JPEG for export, and PNG or JPEG files
//! read as reference images. The `image` crate does the coding (DECISIONS D14); colours are
//! taken as sRGB both ways (colour management is a later slice).

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader};
use openpaint_app::{ExportError, ImageError, ImageFormat, MAX_IMAGE_PIXELS, ReferenceImage};

/// JPEG quality, 1 to 100: high, so ink edges stay clean.
const JPEG_QUALITY: u8 = 92;

/// `rgba` (8-bit RGBA, `size`, rows top first) as a file of `format`. PNG keeps every byte;
/// JPEG drops the alpha channel (the canvas is opaque on its paper).
///
/// # Errors
/// `Encode` when `rgba` does not hold `size` or the encoder refuses it (JPEG sides above 65535).
pub fn encode_image(
    format: ImageFormat,
    size: [u32; 2],
    rgba: &[u8],
) -> Result<Vec<u8>, ExportError> {
    let [width, height] = size;
    if rgba.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return Err(ExportError::Encode);
    }
    let mut out = Vec::new();
    let result = match format {
        ImageFormat::Png => {
            let encoder = PngEncoder::new_with_quality(
                &mut out,
                CompressionType::Default,
                FilterType::Adaptive,
            );
            encoder.write_image(rgba, width, height, ExtendedColorType::Rgba8)
        }
        ImageFormat::Jpeg => {
            let (pixels, _) = rgba.as_chunks::<4>();
            let rgb: Vec<u8> = pixels
                .iter()
                .flat_map(|[r, g, b, _]| [*r, *g, *b])
                .collect();
            JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY).write_image(
                &rgb,
                width,
                height,
                ExtendedColorType::Rgb8,
            )
        }
    };
    result.map_err(|_| ExportError::Encode)?;
    Ok(out)
}

/// The picture in a PNG or JPEG file as 8-bit RGBA at its full size, turned upright as its
/// EXIF orientation says (camera pictures).
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
    let mut picture = DynamicImage::from_decoder(decoder).map_err(decoding)?;
    picture.apply_orientation(orientation);
    let rgba = picture.into_rgba8();
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
