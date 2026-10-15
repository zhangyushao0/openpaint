//! `predict <file.openpaint>` (S58): the tablet's GPU time for the evaluations the device's
//! scenario benchmarks make (`cargo device-input bench open-cold|zoom|strokes`), predicted on the
//! development machine: each scene evaluated here once to bin its plan (`Core::time_evaluation`),
//! its op entries by kind costed per pixel by the tablet's table (S55's, `render/src/cost.rs`:
//! the frame budget's estimate), beside this GPU's measured time. Scenes: `open`, the whole
//! canvas at 1x; `zoom Nx`, the tier's tiles over the densest place (the screen and a ring of one
//! tile) at 2x, 4x and 8x; `stroke <brush>`, the bench's stroke of each texture brush across that
//! place alone on its layer, over its box. Run it with `--release`; the errors against the tablet
//! are in STATUS.

use std::fs;

use openpaint_app::{BrushLibrary, ProjectStore};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_render::{Core, Timed};
use openpaint_runtime::{busiest, drawn, style_of};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::test_art::scratch;

/// The area a canvas fits in under the tablet's top band, in pixels.
const AREA: [f32; 2] = [2880.0, 1716.0];
/// `openpaint_app::FIT_MARGIN`.
const FIT: f32 = 0.92;
/// A view tier's tile, output pixels a side (`render/src/view_tiles.rs`), and the tiles a tier
/// fills over the tablet's screen: those it shows and a ring of one (15 by 11 on the tablet).
const TIER_TILE: u32 = 256;
const TIER_TILES: [u32; 2] = [15, 11];
const REPEATS: usize = 5;
/// The bench's texture brushes (`runtime/src/bench_app_paint.rs`).
const TEXTURES: [&str; 4] = ["textured-round", "flat", "opaque-paint", "speckle"];

/// `sizes`: the texture brushes' sizes as the device's `bench strokes` drew them (its toolbox's),
/// "400,30,400,28"; their first sizes without.
pub fn run(file: &str, sizes: Option<&str>) -> Result<(), HarnessError> {
    let sizes: Vec<Option<f32>> = match sizes {
        Some(sizes) => (sizes.split(','))
            .map(|size| size.parse().map(Some))
            .collect::<Result<_, _>>()
            .map_err(|_| HarnessError::Missing("predict FILE [size,size,size,size]"))?,
        None => vec![None; TEXTURES.len()],
    };
    let artwork = open(file)?;
    let core = Core::headless()?;
    let canvas = artwork.canvas();
    let size = [canvas.width(), canvas.height()];
    let place = busiest(&artwork);
    println!(
        "{file}: {} x {}, densest place ({:.0}, {:.0}) on {}",
        size[0],
        size[1],
        place[0],
        place[1],
        core.adapter()
    );
    println!("scene             tiles    entries | this GPU ms | predicted tablet ms");
    let timed = |at: [u32; 2], scale: u32, area: [u32; 2]| -> Result<Timed, HarnessError> {
        Ok(core
            .time_evaluation(&artwork, (scale, at), area, REPEATS)?
            .0)
    };
    row("open 1x", &timed([0, 0], 1, size)?);
    for scale in [2, 4, 8] {
        let (at, area) = tier_area(size, place, scale);
        row(&format!("zoom {scale}x"), &timed(at, scale, area)?);
    }
    let library = BrushLibrary::built_in()
        .map_err(|_| HarnessError::Missing("the built-in brushes did not load"))?;
    let fit = FIT * (AREA[0] / size[0] as f32).min(AREA[1] / size[1] as f32);
    for (k, key) in TEXTURES.into_iter().enumerate() {
        let path: Vec<[f32; 2]> = (0..90)
            .map(|i| {
                let t = i as f32 / 89.0;
                let y = 60.0 * (t * std::f32::consts::TAU).sin() + 40.0 * k as f32 - 60.0;
                [place[0] + (900.0 * t - 450.0) / fit, place[1] + y / fit]
            })
            .collect();
        let brush = sizes.get(k).copied().flatten();
        let style = style_of(&library, key, brush)?.with_seed(k as u32);
        let alone = Artwork::new(CanvasSize::new(size[0], size[1])?);
        let stroked = drawn(&alone, style.clone(), &path, |_| 0.8)?;
        let (at, area) = stroke_box(&path, style.size(), size);
        let timed = core.time_evaluation(&stroked, (1, at), area, REPEATS)?.0;
        row(&format!("stroke {key} {:.0}", style.size()), &timed);
    }
    Ok(())
}

/// The artwork of an OpenPaint file, read through a library of its own.
fn open(file: &str) -> Result<Artwork, HarnessError> {
    let bytes = fs::read(file)?;
    let dir = scratch("predict-library");
    let artwork = {
        let mut library = Library::at(&dir);
        let id = library.import_file(&bytes).map_err(HarnessError::Port)?;
        library.load(id).map_err(HarnessError::Port)?.artwork
    };
    fs::remove_dir_all(&dir)?;
    Ok(artwork)
}

/// The canvas pixel and output size a tier at `scale` over `place` evaluates: the tiles it shows
/// and a ring of one, within the canvas.
fn tier_area(canvas: [u32; 2], place: [f32; 2], scale: u32) -> ([u32; 2], [u32; 2]) {
    let mut at = [0; 2];
    let mut area = [0; 2];
    for axis in 0..2 {
        let wanted = TIER_TILES[axis] * TIER_TILE;
        let whole = canvas[axis] * scale;
        let side = wanted.min(whole);
        let half = side / (2 * scale);
        let left = (place[axis] as u32).saturating_sub(half);
        let left = left.min(canvas[axis] - side / scale);
        at[axis] = left;
        area[axis] = side;
    }
    (at, area)
}

/// The canvas box a stroke along `path` of diameter `size` covers, within the canvas.
fn stroke_box(path: &[[f32; 2]], size: f32, canvas: [u32; 2]) -> ([u32; 2], [u32; 2]) {
    let mut at = [0; 2];
    let mut area = [0; 2];
    for axis in 0..2 {
        let values = path.iter().map(|p| p[axis]);
        let (low, high) = values.fold((f32::MAX, f32::MIN), |(l, h), v| (l.min(v), h.max(v)));
        let low = (low - size).floor().max(0.0) as u32;
        let high = ((high + size).ceil() as u32).min(canvas[axis]);
        at[axis] = low;
        area[axis] = high.saturating_sub(low).max(1);
    }
    (at, area)
}

fn row(scene: &str, timed: &Timed) {
    println!(
        "{scene:<22} {:>6} {:>10} | {:>11.2} | {:>10.1}",
        timed.tiles,
        timed.entries,
        timed.median() / 1e6,
        timed.estimate / 1e6
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tiers_area_is_its_tiles_within_the_canvas() {
        // On a screen-size canvas at 2x over its middle: 15 by 11 tiles of 256 output pixels.
        let (at, area) = tier_area([2880, 1716], [1440.0, 858.0], 2);
        assert_eq!(area, [3840, 2816]);
        assert_eq!(at, [1440 - 960, 858 - 704]);
        // Near the corner the area stays on the canvas; a small canvas is taken whole.
        let (at, area) = tier_area([2880, 1716], [10.0, 1700.0], 4);
        assert_eq!(at, [0, 1716 - 2816 / 4]);
        assert_eq!(
            tier_area([400, 300], [200.0, 150.0], 2),
            ([0, 0], [800, 600])
        );
        let (at, area_box) = stroke_box(&[[100.0, 50.0], [300.0, 80.0]], 20.0, [250, 1000]);
        assert_eq!((at, area_box), ([80, 30], [170, 70]));
        assert_eq!(area[0], 3840);
    }

    #[test]
    fn the_prediction_is_the_tablets_cost_a_pixel_times_the_pixels() {
        // An empty canvas lists no op: each 16-px tile costs its loop, 1 ns a pixel (`cost.rs`).
        let core = Core::headless().expect("a GPU");
        let artwork = Artwork::new(CanvasSize::new(512, 512).expect("size"));
        let timed = core
            .time_evaluation(&artwork, (1, [0, 0]), [256, 256], 1)
            .expect("timed")
            .0;
        assert_eq!(timed.tiles, 256);
        assert!(
            (timed.estimate - 256.0 * 256.0).abs() < 1.0,
            "{}",
            timed.estimate
        );
    }
}
