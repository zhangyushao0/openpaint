//! `export-check <library dir> [scale]` (S59): each artwork of a library (a scenario's, or a copy
//! of one) exported at 1x, 2x and 4x (or `scale`) as the app exports it, against the evaluator's
//! image of the same output (what exports were before S59, evaluated in pieces): both images'
//! hashes (`RgbaImage::content_hash`, the scenario's `exported` hash) and how far apart they lie
//! (`image_difference`: the largest and mean difference, the pixels off by more than one 8-bit
//! level and of them those away from antialiased edges). Opens the library through the store as
//! the app does, so point it at a copy or a scenario's own.

use std::path::Path;

use openpaint_app::ProjectStore;
use openpaint_core::{Artwork, Color};
use openpaint_render::{Banding, Core, RgbaImage, image_difference};
use openpaint_store::Library;

use crate::error::HarnessError;

/// The evaluator's pieces, output pixels a side (a multiple of every scale).
const PIECE: [u32; 2] = [2048, 1024];

pub fn run(dir: &str, scale: Option<&str>) -> Result<(), HarnessError> {
    if !Path::new(dir).is_dir() {
        return Err(HarnessError::Missing(
            "export-check needs a library directory",
        ));
    }
    let scales: Vec<u32> = match scale {
        Some(scale) => vec![(scale.parse()).map_err(|_| HarnessError::Missing("a scale"))?],
        None => vec![1, 2, 4],
    };
    let core = Core::headless()?;
    println!("adapter {}", core.adapter());
    let mut library = Library::at(dir);
    let listed = library.list().map_err(HarnessError::Port)?;
    for summary in &listed {
        let artwork = library
            .load(summary.id)
            .map_err(HarnessError::Port)?
            .artwork;
        for &scale in &scales {
            let rastered = core
                .export_bands(&artwork, Color::WHITE, scale, Banding::DEFAULT)?
                .image()?;
            let evaluated = evaluated(&core, &artwork, scale)?;
            let found = image_difference(&evaluated, &rastered)
                .ok_or(HarnessError::Missing("the images differ in size"))?;
            let [width, height] = rastered.size();
            println!(
                "artwork {} {width} x {height} at {scale}x: exported {:016x}, evaluated {:016x}; \
                 largest {:?}, mean {:.6}, off by more than one level {} ({} away from edges, \
                 largest there {})",
                summary.id.get(),
                rastered.content_hash(),
                evaluated.content_hash(),
                found.largest,
                found.mean.iter().sum::<f64>() / 4.0,
                found.beyond_one,
                found.beyond_one_inside,
                found.largest_inside
            );
        }
    }
    Ok(())
}

/// The evaluator's image of `artwork`'s whole output at `scale`, in pieces.
fn evaluated(core: &Core, artwork: &Artwork, scale: u32) -> Result<RgbaImage, HarnessError> {
    let canvas = artwork.canvas();
    let [width, height] = [canvas.width(), canvas.height()].map(|side| side * scale);
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    for top in (0..height).step_by(PIECE[1] as usize) {
        for left in (0..width).step_by(PIECE[0] as usize) {
            let size = [PIECE[0].min(width - left), PIECE[1].min(height - top)];
            let at = [left / scale, top / scale];
            let (_, piece) = core.time_evaluation(artwork, (scale, at), size, 1)?;
            for (row, source) in piece
                .pixels()
                .chunks_exact(size[0] as usize * 4)
                .enumerate()
            {
                let start = ((top as usize + row) * width as usize + left as usize) * 4;
                pixels[start..start + source.len()].copy_from_slice(source);
            }
        }
    }
    RgbaImage::from_rgba([width, height], pixels)
        .ok_or(HarnessError::Missing("the pieces do not fill the output"))
}
