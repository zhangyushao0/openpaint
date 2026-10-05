//! `stroke-stats <library dir>`: how the pen was sampled in a library's strokes. Opens the
//! library through the store as the app does (an older schema is upgraded in place, the last
//! project loaded is marked open), so point it at a copy, never at a backup. Prints, over every
//! stroke of every artwork (layers and masks): the time between samples, their spacing in
//! canvas pixels, the pen's speed, the turn between consecutive segments by speed, how far the
//! straight segments lie from the circle through each three samples, and gaps (time steps much
//! longer than the stroke's typical one, split into lost samples while moving and pauses).
//! Strokes with one constant pressure (injected by device routines) are counted and left out.

use std::path::Path;

use openpaint_app::ProjectStore;
use openpaint_core::{Artwork, Stroke, StrokeSample};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::quantiles::{median_f32, median_u32, pick, quantiles};

/// Upper bounds of the time histogram in milliseconds; the last bin is open.
const TIME_BINS: [u32; 13] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 33, 100];
/// Upper bounds of the speed bands in canvas pixels per millisecond; the last band is open.
const SPEED_BANDS: [f32; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
/// A step is a gap when it is this many times the stroke's median step and at least `GAP_MS`.
const GAP_FACTOR: u32 = 3;
const GAP_MS: u32 = 12;
/// A gap is samples lost while moving when it spans this many times the stroke's median spacing.
const LOST_SPACING: f32 = 3.0;
/// Segments shorter than this have no direction worth measuring.
const SHORTEST: f32 = 0.5;
/// Steps shown of the stroke with the most samples.
const PATTERN: usize = 40;
/// A sample within this many pixels of its neighbours' chord, at least `CHORD` long, with the
/// pressure of the one before, looks synthesized.
const ON_CHORD: f32 = 0.02;
const CHORD: f32 = 2.0;

/// One joint between two segments: speed around it, the longer segment, the turn and the
/// sagitta of the longer segment on the circle through the three samples.
#[derive(Clone)]
struct Joint {
    speed: f32,
    spacing: f32,
    turn_deg: f32,
    sagitta: f32,
    size: f32,
    place: String,
    /// The samples around it, `[x, y, pressure, ms]` (ms from the first of them).
    window: Vec<[f32; 4]>,
}

/// Samples on each side of a joint shown with the largest sagittas, as a test fixture.
const WINDOW: usize = 4;

#[derive(Default)]
struct Stats {
    strokes: usize,
    injected: usize,
    samples: usize,
    times: Vec<u32>,
    spacings: Vec<f32>,
    speeds: Vec<f32>,
    joints: Vec<Joint>,
    moving_ms: u64,
    moving_steps: u64,
    lost: Vec<String>,
    pauses: usize,
    synthetic: usize,
    /// The first steps of the stroke with the most samples, as milliseconds/pixels.
    pattern: (usize, String),
}

pub fn run(dir: &str) -> Result<(), HarnessError> {
    if !Path::new(dir).is_dir() {
        return Err(HarnessError::Missing(
            "stroke-stats needs a library directory",
        ));
    }
    let mut library = Library::at(dir);
    let listed = library.list().map_err(HarnessError::Port)?;
    let mut stats = Stats::default();
    let mut strokes: Vec<Stroke> = Vec::new();
    for summary in &listed {
        let project = library.load(summary.id).map_err(HarnessError::Port)?;
        let title = summary.title.as_str();
        let canvas = project.artwork.canvas();
        println!(
            "artwork {} \"{title}\": {} x {}, {} layers",
            summary.id.get(),
            canvas.width(),
            canvas.height(),
            project.artwork.layers().len()
        );
        collect(&project.artwork, summary.id.get(), &mut stats);
        for layer in project.artwork.layers() {
            strokes.extend(layer.strokes().iter().cloned());
            strokes.extend(
                layer
                    .mask()
                    .into_iter()
                    .flat_map(|mask| mask.strokes().iter().cloned()),
            );
        }
    }
    report(&mut stats);
    crate::export_timing::pieces(strokes.iter());
    Ok(())
}

fn collect(artwork: &Artwork, id: u64, stats: &mut Stats) {
    for (index, layer) in artwork.layers().iter().enumerate() {
        let masks = layer.mask().map(|mask| mask.strokes());
        let lists = std::iter::once((layer.strokes(), "")).chain(masks.map(|list| (list, " mask")));
        for (list, on) in lists {
            for (number, stroke) in list.iter().enumerate() {
                let place = format!("artwork {id} layer {index}{on} stroke {number}");
                stroke_stats(stroke, &place, stats);
            }
        }
    }
}

fn stroke_stats(stroke: &Stroke, place: &str, stats: &mut Stats) {
    let samples = stroke.samples();
    if samples.len() < 2 {
        return;
    }
    let first = samples[0].pressure();
    if samples.iter().all(|sample| sample.pressure() == first) {
        stats.injected += 1;
        return;
    }
    stats.strokes += 1;
    stats.samples += samples.len();
    let steps: Vec<(u32, f32)> = samples
        .windows(2)
        .map(|pair| {
            let ms = pair[1].time_ms().saturating_sub(pair[0].time_ms());
            (ms, distance(&pair[0], &pair[1]))
        })
        .collect();
    if samples.len() > stats.pattern.0 {
        let shown = steps
            .iter()
            .take(PATTERN)
            .map(|(ms, px)| format!("{ms}/{px:.1}"));
        stats.pattern = (
            samples.len(),
            format!("{place}: {}", shown.collect::<Vec<_>>().join(" ")),
        );
    }
    step_stats(&steps, place, stats);
    for index in 1..samples.len() - 1 {
        let around = [&samples[index - 1], &samples[index], &samples[index + 1]];
        stats.synthetic += usize::from(synthetic(around));
        if let Some(mut joint) = joint(around, stroke.style().size()) {
            joint.place = format!("{place} sample {index}");
            let around =
                &samples[index.saturating_sub(WINDOW)..(index + WINDOW + 1).min(samples.len())];
            let start = around[0].time_ms();
            joint.window = around
                .iter()
                .map(|s| {
                    let p = s.position();
                    [p.x(), p.y(), s.pressure(), (s.time_ms() - start) as f32]
                })
                .collect();
            stats.joints.push(joint);
        }
    }
}

/// One stroke's steps (milliseconds, pixels): the histograms, gaps and the time drawn.
fn step_stats(steps: &[(u32, f32)], place: &str, stats: &mut Stats) {
    let typical_ms = median_u32(
        steps
            .iter()
            .map(|step| step.0)
            .filter(|ms| *ms > 0)
            .collect(),
    );
    let typical_px = median_f32(steps.iter().map(|step| step.1).collect());
    for (index, &(ms, px)) in steps.iter().enumerate() {
        stats.times.push(ms);
        stats.spacings.push(px);
        if ms > 0 {
            stats.speeds.push(px / ms as f32);
        }
        if ms >= GAP_MS && ms >= GAP_FACTOR * typical_ms {
            if px > LOST_SPACING * typical_px.max(SHORTEST) {
                stats.lost.push(format!(
                    "{place} sample {}: {ms} ms (typical {typical_ms}), {px:.1} px (typical \
                     {typical_px:.1})",
                    index + 1
                ));
            } else {
                stats.pauses += 1;
            }
        } else {
            stats.moving_ms += u64::from(ms);
            stats.moving_steps += 1;
        }
    }
}

/// True for a sample that looks interpolated rather than reported: it has the pressure of the
/// one before and lies on the chord between its neighbours (Android's batched input resamples
/// that way: positions linear, other axes copied).
fn synthetic([a, b, c]: [&StrokeSample; 3]) -> bool {
    let chord = distance(a, c);
    if chord < CHORD || b.pressure() != a.pressure() {
        return false;
    }
    let (p, q, x) = (a.position(), c.position(), b.position());
    let [dx, dy] = [q.x() - p.x(), q.y() - p.y()];
    let [vx, vy] = [x.x() - p.x(), x.y() - p.y()];
    let along = (dx * vx + dy * vy) / (chord * chord);
    (dx * vy - dy * vx).abs() / chord < ON_CHORD && (0.0..=1.0).contains(&along)
}

fn joint([a, b, c]: [&StrokeSample; 3], size: f32) -> Option<Joint> {
    let (ab, bc) = (distance(a, b), distance(b, c));
    if ab < SHORTEST || bc < SHORTEST {
        return None;
    }
    let ms = c.time_ms().checked_sub(a.time_ms()).filter(|ms| *ms > 0)?;
    let [u, v] = [direction(a, b), direction(b, c)];
    let turn = (u[0] * v[1] - u[1] * v[0])
        .atan2(u[0] * v[0] + u[1] * v[1])
        .abs();
    let ac = distance(a, c);
    let longer = ab.max(bc);
    // The circle through the three samples: R = abc / (4 area), the sagitta of the longer chord.
    let area = 0.5 * (ab * bc * turn.sin()).abs();
    let sagitta = if area < 1e-6 {
        0.0
    } else {
        let radius = ab * bc * ac / (4.0 * area);
        radius - (radius * radius - longer * longer / 4.0).max(0.0).sqrt()
    };
    Some(Joint {
        speed: (ab + bc) / ms as f32,
        spacing: longer,
        turn_deg: turn.to_degrees(),
        sagitta,
        size,
        place: String::new(),
        window: Vec::new(),
    })
}

fn distance(a: &StrokeSample, b: &StrokeSample) -> f32 {
    let (p, q) = (a.position(), b.position());
    (q.x() - p.x()).hypot(q.y() - p.y())
}

fn direction(a: &StrokeSample, b: &StrokeSample) -> [f32; 2] {
    let (p, q) = (a.position(), b.position());
    let length = distance(a, b);
    [(q.x() - p.x()) / length, (q.y() - p.y()) / length]
}

fn report(stats: &mut Stats) {
    println!(
        "\n{} pen strokes, {} samples ({} strokes of constant pressure left out: injected)",
        stats.strokes, stats.samples, stats.injected
    );
    if stats.times.is_empty() {
        return;
    }
    let rate = stats.moving_steps as f64 * 1000.0 / stats.moving_ms.max(1) as f64;
    let reported = (stats.moving_steps as f64 - stats.synthetic as f64) * 1000.0
        / stats.moving_ms.max(1) as f64;
    println!(
        "samples per second over steps that are no gaps: {rate:.1} Hz, without the \
         synthesized-looking ones {reported:.1} Hz"
    );
    println!("first steps (ms/px) of {}", stats.pattern.1);
    println!(
        "synthesized-looking samples (the previous pressure, on their neighbours' chord): {} \
         ({:.1} per second of drawing)",
        stats.synthetic,
        stats.synthetic as f64 * 1000.0 / stats.moving_ms.max(1) as f64
    );
    time_histogram(&stats.times);
    let spacings = std::mem::take(&mut stats.spacings);
    let speeds = std::mem::take(&mut stats.speeds);
    println!("spacing px:   {}", quantiles(spacings));
    println!("speed px/ms:  {}", quantiles(speeds));
    bands(&stats.joints);
    println!(
        "\ngaps: {} with samples lost while moving, {} pauses",
        stats.lost.len(),
        stats.pauses
    );
    for line in stats.lost.iter().take(12) {
        println!("  {line}");
    }
    let mut worst = stats.joints.clone();
    worst.sort_by(|a, b| b.sagitta.total_cmp(&a.sagitta));
    println!("\nlargest sagittas (canvas px; times 8 for screen px at 8x):");
    largest(worst.iter());
    println!("largest sagittas faster than {FAST} px/ms:");
    largest(worst.iter().filter(|joint| joint.speed >= FAST));
    println!("their samples, [x, y, pressure, ms]:");
    for joint in worst.iter().filter(|joint| joint.speed >= FAST).take(10) {
        let window: Vec<String> = joint
            .window
            .iter()
            .map(|[x, y, p, ms]| format!("[{x}, {y}, {p}, {ms}]"))
            .collect();
        println!("  [{}],", window.join(", "));
    }
}

/// Joints at least this fast (canvas pixels per millisecond) are listed on their own.
const FAST: f32 = 4.0;

fn largest<'a>(joints: impl Iterator<Item = &'a Joint>) {
    for joint in joints.take(10) {
        println!(
            "  {:.2} px at {:.2} px/ms, spacing {:.1}, turn {:.1} deg, size {:.0}: {}",
            joint.sagitta, joint.speed, joint.spacing, joint.turn_deg, joint.size, joint.place
        );
    }
}

fn time_histogram(times: &[u32]) {
    let mut counts = [0usize; TIME_BINS.len() + 1];
    for ms in times {
        let bin = TIME_BINS
            .iter()
            .position(|top| ms <= top)
            .unwrap_or(TIME_BINS.len());
        counts[bin] += 1;
    }
    let total = times.len() as f64;
    println!("time between samples (ms): count, share");
    let mut low = 0;
    for (bin, count) in counts.iter().enumerate() {
        let label = match TIME_BINS.get(bin) {
            Some(top) if *top == low || bin == 0 => format!("{top}"),
            Some(top) => format!("{low}-{top}"),
            None => format!(">{}", TIME_BINS[TIME_BINS.len() - 1]),
        };
        if let Some(top) = TIME_BINS.get(bin) {
            low = top + 1;
        }
        let share = *count as f64 * 100.0 / total;
        println!("  {label:>7} ms: {count:>7} {share:>5.1} %");
    }
}

/// Joints by speed band: spacing, turn and sagitta quantiles, and the share whose sagitta is
/// over an eighth of a canvas pixel (one screen pixel at 8x).
fn bands(joints: &[Joint]) {
    println!(
        "\nby speed (px/ms): joints | spacing px p50 p90 | turn deg p50 p90 | sagitta px p50 p90 max | > 1/8 px"
    );
    let mut low = 0.0;
    for band in 0..=SPEED_BANDS.len() {
        let high = SPEED_BANDS.get(band).copied().unwrap_or(f32::INFINITY);
        let inside: Vec<&Joint> = joints
            .iter()
            .filter(|joint| joint.speed >= low && joint.speed < high)
            .collect();
        if !inside.is_empty() {
            let spacing = pick(inside.iter().map(|joint| joint.spacing).collect());
            let turn = pick(inside.iter().map(|joint| joint.turn_deg).collect());
            let sagitta = pick(inside.iter().map(|joint| joint.sagitta).collect());
            let over = inside.iter().filter(|joint| joint.sagitta > 0.125).count();
            println!(
                "  {low:>4}-{high:<4} {:>7} | {:>5.1} {:>5.1} | {:>5.1} {:>5.1} | {:>5.2} {:>5.2} {:>6.2} | {:>5.1} %",
                inside.len(),
                spacing[0],
                spacing[1],
                turn[0],
                turn[1],
                sagitta[0],
                sagitta[1],
                sagitta[2],
                over as f64 * 100.0 / inside.len() as f64
            );
        }
        low = high;
    }
}
