//! Where harness results go: PNGs under `artifacts/harness/` at the workspace root (untracked).

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use openpaint_render::RgbaImage;

use crate::error::HarnessError;

/// Writes `image` to `artifacts/harness/<name>`, creating the directory, and returns the path.
pub fn write_png(name: &str, image: &RgbaImage) -> Result<PathBuf, HarnessError> {
    let dir = workspace_root().join("artifacts").join("harness");
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    let [width, height] = image.size();
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(&path)?), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(image.pixels())?;
    writer.finish()?;
    Ok(path)
}

/// The harness lives at `apps/harness`; the workspace root is two levels up.
fn workspace_root() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.ancestors().nth(2).unwrap_or(manifest)
}
