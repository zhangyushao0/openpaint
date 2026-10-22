//! `paint-timing`: what a frame of painting costs over a layer that already holds many strokes.
//! Per brush at its default size (Textured Round, Opaque Paint, Speckle, Flat, and the Ink Pen as
//! the baseline), a 2880 x 1716 canvas (the tablet's screen size) gets N earlier strokes of that
//! brush (0, 100, 300, 600; 52 samples each, 8 ms apart, pressed 0.8, as `cargo device-input
//! pen-many` lays them on the tablet, whose lines it prints) crossing one 800 x 800 px area;
//! then one more stroke is drawn over that area a sample a frame through the in-progress path,
//! as the runtime draws it: calm (52 samples about 12 px apart, a wave) and fast (52 samples
//! 25 px apart, a loop). Per case and frame: the CPU's part (scene, plan, encoding, submission)
//! and the GPU's (waited for); printed as p50 / p95 over the frames after the first, with the
//! first frame (the pen touching down) and the lift's frame apart. Run with `--release` for
//! numbers that mean anything.

use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use openpaint_app::BrushLibrary;
use openpaint_core::{
    ActiveStroke, Artwork, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, take_spent};
use openpaint_runtime::{pile_path, style_of};

use crate::error::HarnessError;

const SIZE: [u32; 2] = [2880, 1716];
/// The area the strokes cross: its centre and half its side.
const CENTRE: [f32; 2] = [1440.0, 858.0];
const HALF: f32 = 400.0;
const SAMPLES: u16 = 52;
const COUNTS: [u32; 4] = [0, 100, 300, 600];
const BRUSHES: [&str; 5] = [
    "textured-round",
    "opaque-paint",
    "speckle",
    "flat",
    "ink-pen",
];

pub fn run() -> Result<(), HarnessError> {
    let core = Core::headless()?;
    println!("adapter {}", core.adapter());
    let library = BrushLibrary::built_in().map_err(openpaint_runtime::RuntimeError::from)?;
    let style = |key: &str, seed: u32| -> Result<StrokeStyle, HarnessError> {
        Ok(style_of(&library, key, None)?.with_seed(seed))
    };
    tablet_lines();
    // Shaders, pipelines and buffers settle on a first stroke nobody counts.
    measure(&core, &blank()?, (style("ink-pen", 1)?, calm()))?;
    println!(
        "{:<15} {:>4} {:<7} | {:>23} | {:>13} {:>15} | {:>23}",
        "brush",
        "N",
        "stroke",
        "first cpu (scene)/gpu",
        "cpu p50/p95",
        "gpu p50/p95",
        "lift cpu (scene)/gpu"
    );
    for key in BRUSHES {
        for count in COUNTS {
            let artwork = layer(count, |seed| style(key, seed))?;
            for (kind, path) in [("calm", calm()), ("fast", fast())] {
                let stroke = (style(key, 0x5EED_0048)?, path);
                let [first, next] = measure(&core, &artwork, stroke)?;
                println!("{key:<15} {count:>4} {kind} 1st | {}", first.row());
                println!("{key:<15} {count:>4} {kind} 2nd | {}", next.row());
            }
        }
    }
    Ok(())
}

fn blank() -> Result<Artwork, HarnessError> {
    Ok(Artwork::new(CanvasSize::new(SIZE[0], SIZE[1])?))
}

/// One frame's cost: CPU (and of it building the scene) and GPU.
#[derive(Clone, Copy)]
struct Frame {
    cpu: Duration,
    scene: Duration,
    gpu: Duration,
}

/// A stroke's frames: the first (the pen touching down), those after it, and the lift's.
struct Frames {
    first: Frame,
    drawn: Vec<Frame>,
    lift: Frame,
}

/// Draws `stroke` over `artwork` a sample a frame into a target that showed `artwork` already,
/// and commits it; then the same path again over the commit. The first is the first stroke over
/// the place since the artwork opened or since an undo (the layer's snapshots hold none of it
/// yet), the second a stroke over a place painted on before.
fn measure(
    core: &Core,
    artwork: &Artwork,
    (style, path): (StrokeStyle, Vec<[f32; 2]>),
) -> Result<[Frames; 2], HarnessError> {
    let samples = path
        .into_iter()
        .zip(0..)
        .map(|(point, i)| sample(point, i))
        .collect::<Result<Vec<_>, _>>()?;
    let mut target = CanvasTarget::new(core, SIZE)?;
    target.draw(core, &request(artwork, None), &[PixelRect::full(SIZE)])?;
    core.wait_idle()?;
    let (one, after) = drawn(core, &mut target, artwork, (&style, &samples))?;
    let again = style.clone().with_seed(style.seed().wrapping_add(1));
    let (two, _) = drawn(core, &mut target, &after, (&again, &samples))?;
    Ok([one, two])
}

fn request<'a>(artwork: &'a Artwork, active: Option<&'a ActiveStroke>) -> FrameRequest<'a> {
    FrameRequest {
        artwork,
        active,
        background: Color::WHITE,
    }
}

/// One stroke drawn a sample a frame over `artwork` and committed: its frames, and the commit.
fn drawn(
    core: &Core,
    target: &mut CanvasTarget,
    artwork: &Artwork,
    (style, samples): (&StrokeStyle, &[StrokeSample]),
) -> Result<(Frames, Artwork), HarnessError> {
    let mut session = artwork.begin_stroke(style.clone(), samples[0])?;
    let mut drawn = Vec::new();
    for at in 0..samples.len() {
        if at > 0 {
            session.extend(&samples[at..=at])?;
        }
        let frame = request(artwork, Some(session.active()));
        drawn.push(timed(core, target, &frame)?);
    }
    let finished = session.finish().artwork;
    let lift = timed(core, target, &request(&finished, None))?;
    let first = drawn.remove(0);
    Ok((Frames { first, drawn, lift }, finished))
}

/// Evaluates `request` into `target`, the CPU's part timed, then waits for the GPU.
fn timed(
    core: &Core,
    target: &mut CanvasTarget,
    request: &FrameRequest<'_>,
) -> Result<Frame, HarnessError> {
    let _ = take_spent();
    let started = Instant::now();
    target.draw(core, request, &[])?;
    let cpu = started.elapsed();
    let scene = Duration::from_nanos(take_spent().scene_ns);
    let waited = Instant::now();
    core.wait_idle()?;
    Ok(Frame {
        cpu,
        scene,
        gpu: waited.elapsed(),
    })
}

impl Frames {
    fn row(&self) -> String {
        let ms = |d: Duration| d.as_secs_f64() * 1e3;
        let spread = |of: &dyn Fn(&Frame) -> f64| {
            let mut values: Vec<f64> = self.drawn.iter().map(of).collect();
            values.sort_by(f64::total_cmp);
            let rank =
                |share: f64| values[((values.len() as f64 * share).ceil() as usize).max(1) - 1];
            [rank(0.5), rank(0.95)]
        };
        let [cpu50, cpu95] = spread(&|f| ms(f.cpu));
        let [gpu50, gpu95] = spread(&|f| ms(f.gpu));
        format!(
            "{:>6.2} ({:>5.2}) / {:>6.2} | {:>5.2} / {:>5.2} {:>6.2} / {:>6.2} | {:>6.2} \
             ({:>5.2}) / {:>6.2}",
            ms(self.first.cpu),
            ms(self.first.scene),
            ms(self.first.gpu),
            cpu50,
            cpu95,
            gpu50,
            gpu95,
            ms(self.lift.cpu),
            ms(self.lift.scene),
            ms(self.lift.gpu),
        )
    }
}

/// `count` strokes of the style `style(seed)` makes: `pen-many`'s path through the area's
/// centre, turned and moved as it lays it on the tablet (`pile_path`), crossing the area.
fn layer(
    count: u32,
    style: impl Fn(u32) -> Result<StrokeStyle, HarnessError>,
) -> Result<Artwork, HarnessError> {
    let mut artwork = blank()?;
    for k in 0..count {
        let samples = pile_path(k, CENTRE)
            .into_iter()
            .zip(0..)
            .map(|(point, i)| sample(point, i))
            .collect::<Result<Vec<_>, _>>()?;
        let mut session = artwork.begin_stroke(style(k)?, samples[0])?;
        session.extend(&samples[1..])?;
        artwork = session.finish().artwork;
    }
    Ok(artwork)
}

/// The calm stroke: a wave across the area, samples about 12 px apart.
fn calm() -> Vec<[f32; 2]> {
    points(|t| {
        let x = CENTRE[0] - 300.0 + 600.0 * t;
        [x, CENTRE[1] + 120.0 * (TAU * t).sin()]
    })
}

/// The fast stroke: a loop inside the area, samples 25 px apart.
fn fast() -> Vec<[f32; 2]> {
    let radius = 25.0 * f32::from(SAMPLES) / TAU;
    debug_assert!(radius < HALF);
    (0..SAMPLES)
        .map(|i| {
            let angle = TAU * f32::from(i) / f32::from(SAMPLES);
            [
                CENTRE[0] + radius * angle.cos(),
                CENTRE[1] + radius * angle.sin(),
            ]
        })
        .collect()
}

/// `SAMPLES` points of `at(t)`, `t` from 0 to 1.
fn points(at: impl Fn(f32) -> [f32; 2]) -> Vec<[f32; 2]> {
    (0..SAMPLES)
        .map(|i| at(f32::from(i) / f32::from(SAMPLES - 1)))
        .collect()
}

/// The `i`th sample of a stroke at `point`, pressed 0.8 as the injected pen does, 8 ms after
/// the one before.
fn sample(point: [f32; 2], i: u16) -> Result<StrokeSample, HarnessError> {
    let position = CanvasPoint::new(point[0], point[1])?;
    Ok(StrokeSample::new(
        position,
        0.8,
        0.0,
        0.0,
        8 * u32::from(i),
    )?)
}

/// Screen pixels per canvas pixel at fit on the tablet, and where the canvas's centre shows.
const SCREEN_SCALE: f32 = 0.92;
const SCREEN_CENTRE: [f32; 2] = [1440.0, 941.0];

/// The `cargo device-input` lines that draw these strokes on the tablet, on a new screen-size
/// artwork at fit.
fn tablet_lines() {
    let screen = |path: Vec<[f32; 2]>| {
        let words: Vec<String> = path
            .iter()
            .map(|p| {
                let at = [0, 1]
                    .map(|axis| SCREEN_CENTRE[axis] + SCREEN_SCALE * (p[axis] - CENTRE[axis]));
                format!("{:.0},{:.0}", at[0], at[1])
            })
            .collect();
        words.join(",")
    };
    println!("on the tablet (a new screen-size artwork at fit):");
    println!(
        "  earlier: cargo device-input pen-many <N> {} 0.8",
        screen(pile_path(0, CENTRE))
    );
    println!("  calm:    cargo device-input pen {} 0.8", screen(calm()));
    println!("  fast:    cargo device-input pen {} 0.8", screen(fast()));
}
