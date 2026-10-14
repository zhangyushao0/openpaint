//! The evaluator's micro-benchmark (S55): fixed scenes (`bench_scenes.rs`), each mostly one kind
//! of op, evaluated over one square of 256 x 256 output pixels (256 tiles) at 1x and in a 4x
//! tier on the device's own GPU, each evaluation timed by GPU timestamps
//! (`Core::time_evaluation`). The tablet runs it through the debug build (`cargo device-input
//! bench [scene]`: answered and written to the app's log), the dev machine through the harness
//! (`openpaint-harness bench [scene]`), so a kind's cost on both GPUs is read off one table.
//!
//! A row: the scene and scale; tiles, op entries, the kind's ops a tile lists; the GPU time
//! (median and fastest of 9); ns a tile; ns a pixel per op of the kind, the base scene's time
//! taken off first (the empty one's, or one fill's under smudges, blurs and adjustments); for
//! stamped spans the stamps a pixel tests per op and ns a pixel per stamp; the frame budget's
//! estimate (`cost.rs`, the tiles one after another) over the measured time.

use std::collections::HashMap;

use openpaint_app::BrushLibrary;
use openpaint_render::{Core, Listed, RenderError, Timed};

use crate::bench_scenes::{CANVAS, SCENES, Scene};
use crate::error::RuntimeError;

/// The square evaluated, in output pixels a side.
const SIDE: u32 = 256;
/// Its scales and top-left canvas pixels: the canvas's middle at 1x, its middle 64 canvas
/// pixels at 4x.
const PLACES: [(u32, [u32; 2]); 2] = [
    (1, [(CANVAS - SIDE) / 2; 2]),
    (4, [(CANVAS - SIDE / 4) / 2; 2]),
];
const REPEATS: usize = 9;
const TILE_PIXELS: f64 = 256.0;

/// Runs the scene called `only` (after its base and the empty scene), or every scene: a header
/// line, then a line per scene and scale.
///
/// # Errors
/// An unknown scene; as the evaluator otherwise.
pub fn bench(core: &Core, only: Option<&str>) -> Result<Vec<String>, RuntimeError> {
    let library = BrushLibrary::built_in()?;
    let chosen = match only {
        None => SCENES.iter().collect(),
        Some(name) => chain(name)?,
    };
    let mut lines = vec![format!(
        "bench on {}: {SIDE} x {SIDE} output pixels at 1x over canvas {:?}, at 4x over {:?}",
        core.adapter(),
        PLACES[0].1,
        PLACES[1].1,
    )];
    let mut times: HashMap<(&str, u32), Timed> = HashMap::new();
    for scene in chosen {
        let artwork = scene.artwork(&library)?;
        for (scale, at) in PLACES {
            let timed = core.time_evaluation(&artwork, (scale, at), SIDE, REPEATS)?;
            let base = scene.base.and_then(|name| times.get(&(name, scale)));
            lines.push(row(scene, scale, &timed, base));
            times.insert((scene.name, scale), timed);
        }
    }
    if let Some(timed) = times.values().next() {
        let timer = if timed.stamped {
            "GPU timestamps"
        } else {
            "waited submissions"
        };
        lines.push(format!("timed by {timer}"));
    }
    Ok(lines)
}

/// The empty scene, the scene `name`'s base and the scene itself, each once.
fn chain(name: &str) -> Result<Vec<&'static Scene>, RuntimeError> {
    let find = |name: &str| SCENES.iter().find(|scene| scene.name == name);
    let unknown = RuntimeError::Render(RenderError::Invalid("no such benchmark scene"));
    let scene = find(name).ok_or(unknown)?;
    let mut chain: Vec<&Scene> = [Some("empty"), scene.base]
        .into_iter()
        .flatten()
        .filter_map(find)
        .collect();
    chain.push(scene);
    chain.dedup_by_key(|scene| scene.name);
    Ok(chain)
}

fn row(scene: &Scene, scale: u32, timed: &Timed, base: Option<&Timed>) -> String {
    let ms = |ns: f64| ns / 1e6;
    let median = timed.median();
    let fastest = timed.times.first().copied().unwrap_or(0.0);
    let count = (scene.kind)(timed) - base.map_or(0.0, |b| (scene.kind)(b));
    let spent = median - base.map_or(0.0, Timed::median);
    let tiles = timed.tiles.max(1) as f64;
    let per_op = if count > 0.0 {
        spent / (TILE_PIXELS * count)
    } else {
        0.0
    };
    let listed = &timed.listed;
    let stamps = if listed.stamped > 0 && listed.stamps > 0.0 {
        format!(
            " | {:.1} stamps/op {:.3} ns/px/stamp",
            listed.stamps / listed.stamped as f64,
            spent / (TILE_PIXELS * listed.stamps),
        )
    } else {
        String::new()
    };
    format!(
        "{:<16} {scale}x {:>4} tiles {:>6} entries ({}) {:>6.1} ops/tile | gpu {:>9.3} ms \
         (min {:>9.3}) | {:>10.0} ns/tile {:>8.3} ns/px/op{stamps} | est/gpu {:.2}",
        scene.name,
        timed.tiles,
        timed.entries,
        kinds(listed),
        count / tiles,
        ms(median),
        ms(fastest),
        median / tiles,
        per_op,
        timed.estimate / median.max(1.0),
    )
}

/// The entries by kind, briefly: spans (pieces), stamped (textured), fills, patches, smudges,
/// filters, others.
fn kinds(listed: &Listed) -> String {
    format!(
        "s {} p {:.0} st {} tx {} f {} pa {} sm {} fi {} o {}",
        listed.spans,
        listed.pieces,
        listed.stamped,
        listed.textured,
        listed.fills,
        listed.patches,
        listed.smudges,
        listed.filters,
        listed.others,
    )
}
