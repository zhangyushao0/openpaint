//! `replay`: the S4 walk-through on the dev machine. A recorded pen stroke of 10,000 samples,
//! then a finger swipe, go batch by batch through the full frame loop at the author's tablet
//! size, followed by 20 undo and redo round trips tapped on the side bar (S6, S11); every frame is
//! timed until the GPU is done. Writes the last screen and the canvas,
//! checks the canvas's hash against the golden value and the slowest frame against the
//! budget. The golden is the canvas, not the screen: the evaluator's pixels are the same on
//! every backend (DECISIONS D82), egui's text antialiasing is not.

use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use openpaint_core::{Artwork, Color};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};

use crate::error::HarnessError;
use crate::launch::launch;
use crate::output::write_png;

/// The MovinkPad 14: 2880 x 1800 pixels at density 1.75.
const SIZE: [u32; 2] = [2880, 1800];
const PIXELS_PER_POINT: f32 = 1.75;
const PEN_SAMPLES: u16 = 10_000;
/// A 240 Hz pen read once per frame at 60 frames per second.
const PER_BATCH: usize = 4;
const SAMPLE_MS: i64 = 4;
/// Slowest frame allowed, input to GPU done (ROADMAP S4).
const BUDGET: Duration = Duration::from_millis(8);
/// Hash of `artifacts/harness/replay-canvas.png`. Same on Vulkan and DX12. Re-recorded in S15:
/// since S14 the pen lands through the fit view (0.92), so the scribble is the screen's
/// scribble scaled by 1 / 0.92 about the centre; the finger swipe pans the view, no ink.
const GOLDEN: u64 = 0xc332_0cf5_0b57_27af;
/// Undo and redo round trips after the strokes.
const TRIPS: usize = 20;
/// Centres of the undo and redo buttons at the bottom of the side bar (S11), in window pixels
/// at `SIZE` and `PIXELS_PER_POINT`; the scenarios tap the same points.
const UNDO: [f32; 2] = [70.0, 1271.0];
const REDO: [f32; 2] = [70.0, 1369.0];

pub fn run() -> Result<(), HarnessError> {
    // Opens the library and the canvas and uploads the font atlas; not part of drawing.
    let mut runtime = launch("replay", PIXELS_PER_POINT, SIZE)?;
    runtime.wait_idle()?;
    let recording = [pen_stroke(), finger_swipe()].concat();
    let mut times = Vec::with_capacity(recording.len());
    for batch in &recording {
        times.push(timed_frame(&mut runtime, batch)?);
    }
    let jumps = round_trips(&mut runtime)?;
    let screen = runtime
        .read_offscreen()?
        .ok_or(HarnessError::Missing("the replay drew no frame"))?;
    let canvas = runtime
        .read_canvas()?
        .ok_or(HarnessError::Missing("the replay opened no canvas"))?;
    let artwork = runtime
        .artwork()
        .ok_or(HarnessError::Missing("the replay opened no canvas"))?;
    println!("adapter {}", runtime.adapter());
    println!(
        "strokes committed {} (the pen's; the finger draws nothing)",
        strokes(artwork)
    );
    println!("{TRIPS} undo/redo round trips tapped, each undo empties the artwork");
    if cold(artwork)? != canvas {
        return Err(HarnessError::Missing(
            "the canvas drawn frame by frame differs from a cold evaluation",
        ));
    }
    println!("canvas drawn frame by frame = cold evaluation of the artwork");
    report("drawing", &times);
    report("undo/redo taps", &jumps);
    let path = write_png("replay.png", &screen)?;
    println!(
        "wrote {} (screen, hash {:016x})",
        path.display(),
        screen.content_hash()
    );
    let path = write_png("replay-canvas.png", &canvas)?;
    println!("wrote {} (canvas)", path.display());
    check(&canvas, times.iter().chain(&jumps).max().copied())
}

/// The canvas against the golden hash, the slowest frame against the budget.
fn check(canvas: &RgbaImage, slowest: Option<Duration>) -> Result<(), HarnessError> {
    let hash = canvas.content_hash();
    println!("hash {hash:016x} (golden {GOLDEN:016x})");
    if hash != GOLDEN {
        return Err(HarnessError::Golden {
            expected: GOLDEN,
            actual: hash,
        });
    }
    let slowest = slowest.unwrap_or_default();
    if slowest >= BUDGET {
        return Err(HarnessError::Budget {
            slowest,
            budget: BUDGET,
        });
    }
    Ok(())
}

/// One batch and its frame, timed until the GPU is done.
fn timed_frame(runtime: &mut Runtime, batch: &MotionBatch) -> Result<Duration, HarnessError> {
    let start = Instant::now();
    runtime.motion(batch);
    runtime.frame_offscreen(SIZE)?;
    runtime.wait_idle()?;
    Ok(start.elapsed())
}

/// Twenty finger taps on undo, each followed by one on redo: the scribble disappears and comes
/// back. Returns the frame times of every tap's down and up.
fn round_trips(runtime: &mut Runtime) -> Result<Vec<Duration>, HarnessError> {
    let mut times = Vec::new();
    let mut clock = i64::from(PEN_SAMPLES) * SAMPLE_MS + 2000;
    for _ in 0..TRIPS {
        for at in [UNDO, REDO] {
            let down = PointerSample {
                position: at,
                pressure: 1.0,
                tilt: 0.0,
                orientation: 0.0,
                time_ms: clock,
            };
            let up = PointerSample {
                time_ms: clock + 60,
                ..down
            };
            for batch in batches(ToolKind::Finger, 2, &[down, up]) {
                times.push(timed_frame(runtime, &batch)?);
                // The frames the runtime asks for next, as the host's pacing runs them: the
                // button state the tap changed shows before the next tap.
                for _ in 0..4 {
                    let start = Instant::now();
                    let again = runtime.frame_offscreen(SIZE)?;
                    runtime.wait_idle()?;
                    times.push(start.elapsed());
                    if !again {
                        break;
                    }
                }
            }
            clock += 200;
            let expected = usize::from(at == REDO);
            if runtime.artwork().map(strokes) != Some(expected) {
                return Err(HarnessError::Missing("a tap on undo or redo did not act"));
            }
        }
    }
    Ok(times)
}

fn strokes(artwork: &Artwork) -> usize {
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

/// `artwork` evaluated whole into a fresh target.
fn cold(artwork: &Artwork) -> Result<RgbaImage, HarnessError> {
    let core = Core::headless()?;
    let canvas = artwork.canvas();
    let size = [canvas.width(), canvas.height()];
    let mut target = CanvasTarget::new(&core, size)?;
    let request = FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    };
    RegionEvaluator::new(&core, &mut target).evaluate(&request, &[PixelRect::full(size)])?;
    Ok(target.read(&core)?)
}

/// Frame count, mean, percentiles and the slowest frame, in milliseconds.
fn report(what: &str, times: &[Duration]) {
    println!("{what}:");
    let mut order: Vec<usize> = (0..times.len()).collect();
    order.sort_by_key(|&index| std::cmp::Reverse(times[index]));
    let worst: Vec<String> = order
        .iter()
        .take(5)
        .map(|&index| format!("#{index} {:.3}", times[index].as_secs_f64() * 1000.0))
        .collect();
    println!("slowest frames (ms) {}", worst.join(", "));
    let mut sorted = times.to_vec();
    sorted.sort();
    let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
    let at = |fraction: f64| {
        let index = ((sorted.len() as f64 - 1.0) * fraction).round() as usize;
        sorted.get(index).copied().map_or(0.0, ms)
    };
    let total: Duration = sorted.iter().sum();
    let mean = ms(total) / sorted.len().max(1) as f64;
    println!(
        "frames {} | mean {mean:.3} ms | p50 {:.3} | p95 {:.3} | p99 {:.3} | max {:.3} | budget {:.0}",
        sorted.len(),
        at(0.5),
        at(0.95),
        at(0.99),
        at(1.0),
        ms(BUDGET)
    );
}

/// A long scribble over the canvas: a drifting 3:2 Lissajous figure, pressure and tilt
/// changing along it, about 7 pixels between samples.
fn pen_stroke() -> Vec<MotionBatch> {
    let samples: Vec<PointerSample> = (0..PEN_SAMPLES)
        .map(|i| {
            let t = f32::from(i) / f32::from(PEN_SAMPLES - 1);
            let angle = TAU * 4.0 * t;
            let reach = 0.55 + 0.4 * (angle / 9.0).cos();
            PointerSample {
                position: [
                    1440.0 + 1250.0 * reach * (3.0 * angle).sin(),
                    942.0 + 760.0 * reach * (2.0 * angle + 0.3).sin(),
                ],
                pressure: 0.35 + 0.3 * (1.0 + (5.0 * angle).sin()),
                tilt: 0.5 + 0.3 * (angle / 3.0).sin(),
                orientation: (angle / 2.0).sin(),
                time_ms: i64::from(i) * SAMPLE_MS,
            }
        })
        .collect();
    batches(ToolKind::Stylus, 0, &samples)
}

/// A finger dragged across the middle of the canvas; it must leave no ink.
fn finger_swipe() -> Vec<MotionBatch> {
    let start = i64::from(PEN_SAMPLES) * SAMPLE_MS + 500;
    let samples: Vec<PointerSample> = (0..120u8)
        .map(|i| PointerSample {
            position: [200.0 + 20.0 * f32::from(i), 1500.0 - 4.0 * f32::from(i)],
            pressure: 1.0,
            tilt: 0.0,
            orientation: 0.0,
            time_ms: start + i64::from(i) * 8,
        })
        .collect();
    batches(ToolKind::Finger, 1, &samples)
}

/// Down with the first sample, moves of `PER_BATCH` samples, up with the last.
fn batches(tool: ToolKind, id: i32, samples: &[PointerSample]) -> Vec<MotionBatch> {
    let batch = |phase, samples: &[PointerSample]| MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id,
            tool,
            samples: samples.to_vec(),
        }],
    };
    let last = samples.len() - 1;
    let mut out = vec![batch(MotionPhase::Down, &samples[..1])];
    out.extend(
        samples[1..last]
            .chunks(PER_BATCH)
            .map(|chunk| batch(MotionPhase::Move, chunk)),
    );
    out.push(batch(MotionPhase::Up, &samples[last..]));
    out
}
