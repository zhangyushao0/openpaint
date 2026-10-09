//! Gesture steps as the motion batches a host would deliver: one pointer touches down, moves
//! and lifts, sampled like a 240 Hz pen read once per 60 Hz frame.

use openpaint_runtime::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};

use crate::scenario::{Lands, Palm, Tool};

/// The pointer id of a hand beside the pen (the pen is 0).
const PALM_ID: i32 = 1;
/// Milliseconds between samples.
const SAMPLE_MS: i64 = 4;
/// Samples per batch; one batch per frame.
const PER_BATCH: usize = 4;
/// Largest distance between two samples of a path, in pixels.
const SPACING: f32 = 4.0;
/// How long a tap stays down.
const TAP_MS: i64 = 60;

/// A tap at `at`, starting at `start_ms`: down and up at one spot.
pub(crate) fn tap(tool: Tool, at: [f32; 2], start_ms: i64) -> Vec<MotionBatch> {
    let down = sample(at, 1.0, start_ms);
    let up = PointerSample {
        time_ms: start_ms + TAP_MS,
        ..down
    };
    batches(tool, &[down, up])
}

/// The pen hovering over `at` without touching, at `start_ms`: one hover move, as Android
/// reports a stylus held just above the glass.
pub(crate) fn hover(at: [f32; 2], start_ms: i64) -> Vec<MotionBatch> {
    vec![MotionBatch {
        phase: MotionPhase::HoverMove,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id: 0,
            tool: ToolKind::Stylus,
            samples: vec![sample(at, 0.0, start_ms)],
        }],
    }]
}

/// A path through `points` (`[x, y, pressure]`), resampled so no two samples are more than
/// `SPACING` apart, pressure interpolated linearly, starting at `start_ms`.
pub(crate) fn path(tool: Tool, points: &[[f32; 3]], start_ms: i64) -> Vec<MotionBatch> {
    batches(tool, &resampled(points, start_ms, SPACING))
}

/// A pen stroke along `points` as `path` draws it, or (`sparse`) a sample at each point only, as
/// a fast pen reports it, then resting at the last point for `hold_ms` (a sample every
/// `SAMPLE_MS` there, QuickShape). With `lift` the pen leaves the glass there as Android reports
/// it (one more sample, pressure 0). A `palm` touches down beside it as a pointer of its own.
/// The pen leans `lean` degrees (tilt from upright, then its direction) all along (S44).
pub(crate) fn stroke(
    (points, sparse, hold_ms): (&[[f32; 3]], bool, u32),
    lift: Option<[f32; 2]>,
    palm: Option<Palm>,
    lean: [f32; 2],
    start_ms: i64,
) -> Vec<MotionBatch> {
    let spacing = if sparse { f32::INFINITY } else { SPACING };
    let mut samples = resampled(points, start_ms, spacing);
    if let Some(&last) = samples.last() {
        let rest = (1..=i64::from(hold_ms) / SAMPLE_MS).map(|k| PointerSample {
            time_ms: last.time_ms + k * SAMPLE_MS,
            ..last
        });
        samples.extend(rest);
    }
    if let (Some(at), Some(last)) = (lift, samples.last()) {
        samples.push(sample(at, 0.0, last.time_ms + SAMPLE_MS));
    }
    for sample in &mut samples {
        (sample.tilt, sample.orientation) = (lean[0].to_radians(), lean[1].to_radians());
    }
    let mut pen = batches(Tool::Stylus, &samples);
    let Some(palm) = palm.filter(|_| !pen.is_empty()) else {
        return pen;
    };
    let end_ms = end_ms(&pen, start_ms);
    let hand = |phase, time_ms| MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id: PALM_ID,
            tool: ToolKind::Finger,
            samples: vec![sample(palm.at, 1.0, time_ms)],
        }],
    };
    let (down, up) = (
        hand(MotionPhase::Down, start_ms),
        hand(MotionPhase::Up, end_ms),
    );
    match palm.lands {
        Lands::Before => {
            pen.insert(0, down);
            pen.push(up);
        }
        Lands::During => {
            pen.insert(1, down);
            pen.insert(pen.len() - 1, up);
        }
    }
    pen
}

/// One contact through `[x, y, ms]` keys at `pressure`, a sample every `SAMPLE_MS` from
/// `start` (ms), each placed by time between the keys around it: equal places hold it still.
pub(crate) fn press(tool: Tool, keys: &[[f32; 3]], pressure: f32, start: i64) -> Vec<MotionBatch> {
    let Some(end) = keys.last().map(|key| key[2]) else {
        return Vec::new();
    };
    let at = |ms: f32| -> [f32; 2] {
        let next = keys
            .iter()
            .position(|key| key[2] >= ms)
            .unwrap_or(keys.len() - 1);
        let (a, b) = (keys[next.saturating_sub(1)], keys[next]);
        let span = b[2] - a[2];
        let t = if span > 0.0 {
            ((ms - a[2]) / span).clamp(0.0, 1.0)
        } else {
            1.0
        };
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    };
    let count = (end / SAMPLE_MS as f32).ceil() as i64;
    let samples: Vec<PointerSample> = (0..=count)
        .map(|i| sample(at((i * SAMPLE_MS) as f32), pressure, start + i * SAMPLE_MS))
        .collect();
    batches(tool, &samples)
}

/// `points` resampled at most `spacing` apart as `path` describes, one sample every `SAMPLE_MS`
/// from `start_ms`.
fn resampled(points: &[[f32; 3]], start_ms: i64, spacing: f32) -> Vec<PointerSample> {
    let mut vertices = Vec::new();
    for pair in points.windows(2) {
        let ([ax, ay, ap], [bx, by, bp]) = (pair[0], pair[1]);
        let steps = ((bx - ax).hypot(by - ay) / spacing).ceil().max(1.0) as u32;
        vertices.extend((0..steps).map(|k| {
            let t = k as f32 / steps as f32;
            [ax + (bx - ax) * t, ay + (by - ay) * t, ap + (bp - ap) * t]
        }));
    }
    vertices.extend(points.last());
    (0i64..)
        .zip(&vertices)
        .map(|(i, &[x, y, pressure])| sample([x, y], pressure, start_ms + i * SAMPLE_MS))
        .collect()
}

/// Fingers touching down at `from` (one after another in the same instant, as Android reports
/// them), moving straight to `to` over `ms`, sampled every `SAMPLE_MS` and batched per frame,
/// then lifting one after another, the last move batched with the first lift.
pub(crate) fn fingers(
    from: &[[f32; 2]],
    to: &[[f32; 2]],
    ms: u32,
    start_ms: i64,
) -> Vec<MotionBatch> {
    let frames = (i64::from(ms) / (SAMPLE_MS * PER_BATCH as i64)).max(1);
    let samples = frames * PER_BATCH as i64;
    let at = |k: i64| -> Vec<[f32; 2]> {
        let t = k as f32 / samples as f32;
        from.iter()
            .zip(to)
            .map(|(a, b)| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
            .collect()
    };
    let pointers = |ks: &[i64], count: usize| -> Vec<Pointer> {
        (0..count)
            .map(|finger| Pointer {
                id: finger as i32,
                tool: ToolKind::Finger,
                samples: ks
                    .iter()
                    .map(|&k| sample(at(k)[finger], 1.0, start_ms + k * SAMPLE_MS))
                    .collect(),
            })
            .collect()
    };
    let batch = |phase, pointers| MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers,
    };
    let count = from.len().min(to.len());
    let mut out: Vec<MotionBatch> = (0..count)
        .map(|index| match index {
            0 => batch(MotionPhase::Down, pointers(&[0], 1)),
            _ => batch(
                MotionPhase::PointerDown { index },
                pointers(&[0], index + 1),
            ),
        })
        .collect();
    for frame in 0..frames - 1 {
        let ks: Vec<i64> = (1..=PER_BATCH as i64)
            .map(|k| frame * PER_BATCH as i64 + k)
            .collect();
        out.push(batch(MotionPhase::Move, pointers(&ks, count)));
    }
    let last: Vec<i64> = (samples - PER_BATCH as i64 + 1..=samples).collect();
    for index in (0..count).rev() {
        let ks = if index + 1 == count {
            &last[..]
        } else {
            &[samples][..]
        };
        out.push(match index {
            0 => batch(MotionPhase::Up, pointers(ks, 1)),
            _ => batch(MotionPhase::PointerUp { index }, pointers(ks, index + 1)),
        });
    }
    out
}

/// The time of the last sample in `batches`, or `fallback` when there is none.
pub(crate) fn end_ms(batches: &[MotionBatch], fallback: i64) -> i64 {
    batches
        .iter()
        .flat_map(|batch| &batch.pointers)
        .flat_map(|pointer| &pointer.samples)
        .map(|sample| sample.time_ms)
        .max()
        .unwrap_or(fallback)
}

fn sample(position: [f32; 2], pressure: f32, time_ms: i64) -> PointerSample {
    PointerSample {
        position,
        pressure,
        tilt: 0.0,
        orientation: 0.0,
        time_ms,
    }
}

/// Down with the first sample, moves of `PER_BATCH` samples, up with the last. A single
/// sample goes down and up at once.
fn batches(tool: Tool, samples: &[PointerSample]) -> Vec<MotionBatch> {
    let tool = match tool {
        Tool::Finger => ToolKind::Finger,
        Tool::Stylus => ToolKind::Stylus,
    };
    let batch = |phase, samples: &[PointerSample]| MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id: 0,
            tool,
            samples: samples.to_vec(),
        }],
    };
    let Some(last) = samples.len().checked_sub(1) else {
        return Vec::new();
    };
    let mut out = vec![batch(MotionPhase::Down, &samples[..1])];
    if last > 0 {
        out.extend(
            samples[1..last]
                .chunks(PER_BATCH)
                .map(|chunk| batch(MotionPhase::Move, chunk)),
        );
    }
    out.push(batch(MotionPhase::Up, &samples[last..]));
    out
}

#[cfg(test)]
#[path = "gesture_tests.rs"]
mod tests;
