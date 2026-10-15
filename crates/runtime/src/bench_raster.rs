//! The raster core probe's benchmark (S57, `openpaint_render::RasterTimed`): S55's brush scenes
//! (16 strokes of a built-in at its first size, two of Opaque Paint at 400 px) and S48's pile
//! (300 strokes, 60 of each of Ink Pen, Textured Round, Opaque Paint, Speckle and Flat in turn,
//! the path `cargo device-input pen-many` lays, turned and moved per stroke) in the middle of a
//! 2880 x 1800 canvas, each over a full screen of 2880 x 1800 output pixels at 1x, 4x and 8x
//! centred on that middle: the evaluator's GPU time and the probe's (medians of 9 after a warm-up),
//! their ratio, what the probe drew (strokes, batches, passes, instances, its CPU's part) and how
//! far its image lies from the evaluator's. `raster-passes` measures what a render pass costs.
//! Run on the tablet by `cargo device-input bench raster[-scene][@scale][@atlas]`, on the dev
//! machine by `openpaint-harness bench raster...`.

use openpaint_app::BrushLibrary;
use openpaint_core::{Artwork, CanvasSize};
use openpaint_render::{Core, ImageDifference, RasterTimed, RenderError, Timed, image_difference};

use crate::bench_scenes::{drawn, pile_path, style_of, wave};
use crate::error::RuntimeError;

const CANVAS: [u32; 2] = [2880, 1800];
const MIDDLE: [f32; 2] = [1440.0, 900.0];
/// The tablet's screen in output pixels.
const SCREEN: [u32; 2] = [2880, 1800];
const SCALES: [u32; 3] = [1, 4, 8];
const REPEATS: usize = 9;
/// The probe's scratch atlas, pixels a side.
const ATLAS: u32 = 4096;
const PILE: [&str; 5] = [
    "ink-pen",
    "textured-round",
    "opaque-paint",
    "speckle",
    "flat",
];

type Make = fn(&BrushLibrary) -> Result<Artwork, RuntimeError>;

const SCENES: [(&str, Make); 9] = [
    ("ink-pen", |l| brushed(l, "ink-pen", None, 16)),
    ("soft-brush", |l| brushed(l, "soft-brush", None, 16)),
    ("pencil", |l| brushed(l, "pencil", None, 16)),
    ("textured-round", |l| brushed(l, "textured-round", None, 16)),
    ("opaque-paint", |l| brushed(l, "opaque-paint", None, 16)),
    ("opaque-paint-400", |l| {
        brushed(l, "opaque-paint", Some(400.0), 2)
    }),
    ("speckle", |l| brushed(l, "speckle", None, 16)),
    ("flat", |l| brushed(l, "flat", None, 16)),
    ("pile", pile),
];

/// The probe's rows for `only` (after `raster`): every scene, `-name`, `-name@scale`,
/// `-name@scale@atlas` (a number above 8 is the atlas's side; `@alone` times the probe without
/// the evaluator), or `-passes`.
///
/// # Errors
/// An unknown scene; as the evaluator or the probe otherwise.
pub(crate) fn bench_raster(core: &Core, only: &str) -> Result<Vec<String>, RuntimeError> {
    let unknown = || RuntimeError::Render(RenderError::Invalid("no such raster probe scene"));
    let mut words = only.trim_start_matches('-').split('@');
    let name = words.next().unwrap_or("");
    let (alone, words): (Vec<&str>, Vec<&str>) = words.partition(|word| *word == "alone");
    let numbers = (words.iter())
        .map(|word| word.parse::<u32>().map_err(|_| unknown()))
        .collect::<Result<Vec<_>, _>>()?;
    let atlas = numbers.iter().copied().find(|n| *n > 8).unwrap_or(ATLAS);
    let scales: Vec<u32> = match numbers.iter().copied().find(|n| *n <= 8) {
        Some(scale) => vec![scale],
        None => SCALES.to_vec(),
    };
    let mut lines = vec![format!(
        "raster probe on {}: {} x {} output pixels at {scales:?} over the canvas's middle, \
         atlas {atlas} px a side, medians of {REPEATS}",
        core.adapter(),
        SCREEN[0],
        SCREEN[1],
    )];
    if name == "passes" {
        lines.extend(passes(core, atlas)?);
        return Ok(lines);
    }
    let library = BrushLibrary::built_in()?;
    let chosen: Vec<&(&str, Make)> = SCENES
        .iter()
        .filter(|(scene, _)| name.is_empty() || *scene == name)
        .collect();
    if chosen.is_empty() {
        return Err(unknown());
    }
    for (scene, make) in chosen {
        let artwork = make(&library)?;
        for &scale in &scales {
            let at = [0, 1].map(|axis| CANVAS[axis] / 2 - SCREEN[axis] / (2 * scale));
            let evaluated = if alone.is_empty() {
                Some(core.time_evaluation(&artwork, (scale, at), SCREEN, REPEATS)?)
            } else {
                None
            };
            let rastered = core.time_raster(&artwork, (scale, at), SCREEN, (REPEATS, atlas))?;
            if lines.len() == 1 {
                lines.push(setup(&rastered));
            }
            let (timed, difference) = match &evaluated {
                Some((timed, image)) => (timed.clone(), image_difference(image, &rastered.image)),
                None => (Timed::default(), None),
            };
            lines.push(row((scene, scale), &timed, &rastered, difference));
        }
    }
    Ok(lines)
}

/// The probe's targets and how it was timed.
fn setup(rastered: &RasterTimed) -> String {
    let mb = |bytes: u64| bytes as f64 / 1e6;
    format!(
        "atlas {:?} {} ({:.0} MB), layer {} ({:.0} MB), 9 pipelines made in {:.0} ms, timed by {}",
        rastered.atlas,
        rastered.formats[0],
        mb(rastered.bytes[0]),
        rastered.formats[1],
        mb(rastered.bytes[1]),
        rastered.compile_ns / 1e6,
        if rastered.stamped {
            "GPU timestamps"
        } else {
            "waited submissions"
        },
    )
}

fn row(
    (scene, scale): (&str, u32),
    evaluated: &Timed,
    rastered: &RasterTimed,
    difference: Option<ImageDifference>,
) -> String {
    let ms = |ns: f64| ns / 1e6;
    let (eval, raster) = (evaluated.median(), rastered.median());
    let counts = &rastered.counts;
    let diff = difference.map_or("not compared".to_owned(), |d| {
        let [r, g, b, a] = d.mean;
        format!(
            "max {:?} mean {r:.3} {g:.3} {b:.3} {a:.3} | >1 {} inside {} (max {} at {:?}, \
             steps there {})",
            d.largest,
            d.beyond_one,
            d.beyond_one_inside,
            d.largest_inside,
            d.worst_inside,
            d.contrast_inside
        )
    });
    format!(
        "{scene:<16} {scale}x | eval {:>9.2} ms raster {:>8.2} ms x{:>6.1} | strokes {}/{} \
         batches {} passes {} pieces {} stamps {} tiles {} regions {:.1} Mpx | cpu plan {:.1} \
         upload {:.1} encode {:.2} ms | repeatable {} | {diff}",
        ms(eval),
        ms(raster),
        eval / raster.max(1.0),
        counts.strokes,
        counts.scene_strokes,
        counts.batches,
        rastered.passes,
        counts.pieces,
        counts.stamps,
        counts.tiles,
        counts.region_pixels as f64 / 1e6,
        ms(rastered.plan_ns),
        ms(rastered.upload_ns),
        ms(rastered.encode_ns),
        rastered.repeatable,
    )
}

/// What a render pass over each of the probe's attachments costs.
fn passes(core: &Core, atlas: u32) -> Result<Vec<String>, RuntimeError> {
    let costs = core.time_passes(SCREEN, atlas)?;
    if costs.is_empty() {
        return Ok(vec![
            "no GPU timestamps: pass costs not measured".to_owned(),
        ]);
    }
    Ok(costs
        .iter()
        .map(|cost| {
            let us = cost.ns.map(|ns| ns / 1e3);
            // A loaded and stored attachment moves its bytes twice.
            let rate = 2.0 * cost.bytes as f64 / cost.ns[0].max(1.0);
            format!(
                "pass over the {:<6} {:?} {} ({:.1} MB): load+store {:.0} us ({rate:.1} GB/s), \
                 clear+store {:.0} us, clear+discard {:.0} us",
                cost.label,
                cost.size,
                cost.format,
                cost.bytes as f64 / 1e6,
                us[0],
                us[1],
                us[2],
            )
        })
        .collect())
}

fn blank() -> Result<Artwork, RuntimeError> {
    Ok(Artwork::new(CanvasSize::new(CANVAS[0], CANVAS[1])?))
}

/// `count` strokes of built-in `key` at `size` along the wave through the middle.
fn brushed(
    library: &BrushLibrary,
    key: &str,
    size: Option<f32>,
    count: u32,
) -> Result<Artwork, RuntimeError> {
    let style = style_of(library, key, size)?;
    let mut artwork = blank()?;
    for k in 0..count {
        artwork = drawn(
            &artwork,
            style.clone().with_seed(k),
            &wave(k, MIDDLE),
            |_| 0.8,
        )?;
    }
    Ok(artwork)
}

/// S48's pile: stroke `k` of `PILE[k % 5]` at its first size along `pen-many`'s path for `k`.
fn pile(library: &BrushLibrary) -> Result<Artwork, RuntimeError> {
    let styles = (PILE.iter())
        .map(|key| style_of(library, key, None))
        .collect::<Result<Vec<_>, _>>()?;
    let mut artwork = blank()?;
    for k in 0..300u32 {
        let style = styles[k as usize % PILE.len()].clone().with_seed(k);
        artwork = drawn(&artwork, style, &pile_path(k, MIDDLE), |_| 0.8)?;
    }
    Ok(artwork)
}
