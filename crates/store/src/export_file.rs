//! Export files (S13, S27, S29): an artwork's pixels written as PNG or JPEG, declaring sRGB by an
//! embedded profile (PNG `iCCP`, JPEG APP2; D53). The pixels come in bands of whole rows, top
//! first, so a 4x export is never held whole: a PNG streams each band through the png crate's
//! stream writer into the file's bytes; a JPEG (whose encoder takes the picture at once) keeps
//! its RGB rows until the last, which is why the app offers JPEG up to 2x
//! (`ExportScale::fits`).

use std::borrow::Cow;
use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder as _};
use openpaint_app::{ExportError, ImageFormat};

use crate::colour::SRGB_PROFILE;

/// JPEG quality, 1 to 100: high, so ink edges stay clean.
const JPEG_QUALITY: u8 = 92;

/// A file being encoded, one band of rows at a time.
pub struct ImageEncoder {
    size: [u32; 2],
    /// Rows taken so far.
    rows: u32,
    kind: Kind,
}

enum Kind {
    /// The stream writer (large: boxed) and the file's bytes it writes into.
    Png(Box<png::StreamWriter<'static, File>>, File),
    /// The RGB rows so far.
    Jpeg(Vec<u8>),
}

/// The file's bytes, shared with the PNG writer, which owns its output until it finishes.
#[derive(Clone, Default)]
struct File(Arc<Mutex<Vec<u8>>>);

impl File {
    fn take(&self) -> Vec<u8> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Write for File {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut file = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        file.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ImageEncoder {
    /// Starts a file of `format` holding `size` pixels.
    ///
    /// # Errors
    /// `Encode` for an empty size or a PNG header the encoder refuses.
    pub fn new(format: ImageFormat, size: [u32; 2]) -> Result<Self, ExportError> {
        let [width, height] = size;
        if width == 0 || height == 0 {
            return Err(ExportError::Encode);
        }
        let kind = match format {
            ImageFormat::Png => {
                let file = File::default();
                let mut info = png::Info::with_size(width, height);
                info.icc_profile = Some(Cow::Borrowed(SRGB_PROFILE));
                let mut encoder = png::Encoder::with_info(file.clone(), info).map_err(encode)?;
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.set_compression(png::Compression::Balanced);
                encoder.set_filter(png::Filter::Adaptive);
                let writer = encoder.write_header().map_err(encode)?;
                Kind::Png(Box::new(writer.into_stream_writer().map_err(encode)?), file)
            }
            ImageFormat::Jpeg => Kind::Jpeg(Vec::new()),
        };
        Ok(Self {
            size,
            rows: 0,
            kind,
        })
    }

    /// The next whole rows, 8-bit sRGB RGBA, top first. JPEG drops the alpha channel (the
    /// canvas is opaque on its paper).
    ///
    /// # Errors
    /// `Encode` for a part of a row, rows beyond the picture's height, or a failed write.
    pub fn rows(&mut self, rgba: &[u8]) -> Result<(), ExportError> {
        let row = u64::from(self.size[0]) * 4;
        let bytes = rgba.len() as u64;
        let count = u32::try_from(bytes / row).map_err(|_| ExportError::Encode)?;
        if !bytes.is_multiple_of(row) || self.rows.saturating_add(count) > self.size[1] {
            return Err(ExportError::Encode);
        }
        match &mut self.kind {
            Kind::Png(writer, _) => writer.write_all(rgba).map_err(|_| ExportError::Encode)?,
            Kind::Jpeg(rgb) => {
                let (pixels, _) = rgba.as_chunks::<4>();
                rgb.extend(pixels.iter().flat_map(|[r, g, b, _]| [*r, *g, *b]));
            }
        }
        self.rows += count;
        Ok(())
    }

    /// Ends the file once every row was given; its bytes.
    ///
    /// # Errors
    /// `Encode` when rows are missing or the encoder fails (JPEG sides above 65535).
    pub fn finish(self) -> Result<Vec<u8>, ExportError> {
        if self.rows != self.size[1] {
            return Err(ExportError::Encode);
        }
        match self.kind {
            Kind::Png(writer, file) => {
                // Finishing drops the writer, which ends the file with its last chunk.
                writer.finish().map_err(encode)?;
                Ok(file.take())
            }
            Kind::Jpeg(rgb) => {
                let [width, height] = self.size;
                let mut out = Vec::new();
                let mut encoder = JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
                encoder
                    .set_icc_profile(SRGB_PROFILE.to_vec())
                    .map_err(|_| ExportError::Encode)?;
                encoder
                    .write_image(&rgb, width, height, ExtendedColorType::Rgb8)
                    .map_err(|_| ExportError::Encode)?;
                Ok(out)
            }
        }
    }
}

fn encode(_: png::EncodingError) -> ExportError {
    ExportError::Encode
}

/// `rgba` (8-bit sRGB RGBA, `size`, rows top first) as a whole file of `format`: one band.
///
/// # Errors
/// `Encode` when `rgba` does not hold `size` or the encoder refuses it.
pub fn encode_image(
    format: ImageFormat,
    size: [u32; 2],
    rgba: &[u8],
) -> Result<Vec<u8>, ExportError> {
    let [width, height] = size;
    if rgba.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return Err(ExportError::Encode);
    }
    let mut encoder = ImageEncoder::new(format, size)?;
    encoder.rows(rgba)?;
    encoder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_image;

    /// 5 x 7 pixels, every one different, some translucent.
    fn picture() -> Vec<u8> {
        (0..35u8)
            .flat_map(|i| [i * 7, 255 - i * 3, i * 5, 128 + i])
            .collect()
    }

    #[test]
    fn a_png_given_in_bands_equals_the_one_given_whole() {
        let pixels = picture();
        let whole = encode_image(ImageFormat::Png, [5, 7], &pixels).expect("whole");
        let mut encoder = ImageEncoder::new(ImageFormat::Png, [5, 7]).expect("new");
        for band in pixels.chunks(5 * 4 * 3) {
            encoder.rows(band).expect("rows");
        }
        let banded = encoder.finish().expect("finish");
        assert_eq!(banded, whole, "the same file");
        assert_eq!(decode_image(&banded), Ok(([5, 7], pixels)));
    }

    #[test]
    fn rows_must_be_whole_and_complete() {
        let pixels = picture();
        let mut encoder = ImageEncoder::new(ImageFormat::Jpeg, [5, 7]).expect("new");
        assert_eq!(
            encoder.rows(&pixels[..6]),
            Err(ExportError::Encode),
            "part of a row"
        );
        encoder.rows(&pixels[..20]).expect("one row");
        assert_eq!(encoder.finish(), Err(ExportError::Encode), "rows missing");
        let mut encoder = ImageEncoder::new(ImageFormat::Png, [5, 7]).expect("new");
        encoder.rows(&pixels).expect("all");
        assert_eq!(
            encoder.rows(&pixels[..20]),
            Err(ExportError::Encode),
            "too many"
        );
    }
}
