//! Where harness inputs and results live: scenarios under `tests/scenarios/`, PNGs under
//! `artifacts/` at the workspace root (untracked).

use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use openpaint_render::RgbaImage;

use crate::error::HarnessError;

/// Writes `image` to `artifacts/harness/<name>`, creating the directory, and returns the path.
pub fn write_png(name: &str, image: &RgbaImage) -> Result<PathBuf, HarnessError> {
    let dir = workspace_root().join("artifacts").join("harness");
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    write_rgba(&path, image.size(), image.pixels())?;
    Ok(path)
}

/// Writes 8-bit RGBA pixels of `size` as a PNG at `path`; the directory must exist.
pub fn write_rgba(path: &Path, size: [u32; 2], pixels: &[u8]) -> Result<(), HarnessError> {
    let [width, height] = size;
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)?;
    writer.finish()?;
    Ok(())
}

/// Reads a PNG this harness wrote: 8-bit RGBA, one frame. `None` when the file is missing or
/// is not such a PNG; the caller then has no reference to compare against.
pub fn read_rgba(path: &Path) -> Option<([u32; 2], Vec<u8>)> {
    let file = File::open(path).ok()?;
    let mut reader = png::Decoder::new(BufReader::new(file)).read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut pixels).ok()?;
    let rgba8 = info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight;
    pixels.truncate(info.buffer_size());
    rgba8.then_some(([info.width, info.height], pixels))
}

/// The harness lives at `apps/harness`; the workspace root is two levels up.
pub fn workspace_root() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.ancestors().nth(2).unwrap_or(manifest)
}
