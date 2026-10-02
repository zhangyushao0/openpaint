//! Gesture steps as the motion batches a host would deliver: one pointer touches down, moves
//! and lifts, sampled like a 240 Hz pen read once per 60 Hz frame.

use openpaint_runtime::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};

use crate::scenario::Tool;

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

/// A path through `points` (`[x, y, pressure]`), resampled so no two samples are more than
/// `SPACING` apart, pressure interpolated linearly, starting at `start_ms`.
pub(crate) fn path(tool: Tool, points: &[[f32; 3]], start_ms: i64) -> Vec<MotionBatch> {
    let mut vertices = Vec::new();
    for pair in points.windows(2) {
        let ([ax, ay, ap], [bx, by, bp]) = (pair[0], pair[1]);
        let steps = ((bx - ax).hypot(by - ay) / SPACING).ceil().max(1.0) as u32;
        vertices.extend((0..steps).map(|k| {
            let t = k as f32 / steps as f32;
            [ax + (bx - ax) * t, ay + (by - ay) * t, ap + (bp - ap) * t]
        }));
    }
    vertices.extend(points.last());
    let samples: Vec<PointerSample> = (0i64..)
        .zip(&vertices)
        .map(|(i, &[x, y, pressure])| sample([x, y], pressure, start_ms + i * SAMPLE_MS))
        .collect();
    batches(tool, &samples)
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
mod tests {
    use super::*;

    fn phases(batches: &[MotionBatch]) -> Vec<MotionPhase> {
        batches.iter().map(|batch| batch.phase).collect()
    }

    #[test]
    fn a_tap_goes_down_and_up_at_one_spot() {
        let batches = tap(Tool::Finger, [10.0, 20.0], 100);
        assert_eq!(phases(&batches), [MotionPhase::Down, MotionPhase::Up]);
        let up = &batches[1].pointers[0];
        assert_eq!(up.tool, ToolKind::Finger);
        assert_eq!(up.samples[0].position, [10.0, 20.0]);
        assert_eq!(end_ms(&batches, 0), 100 + TAP_MS);
    }

    #[test]
    fn a_path_is_resampled_and_batched_per_frame() {
        let batches = path(Tool::Stylus, &[[0.0, 0.0, 0.2], [40.0, 0.0, 0.6]], 1000);
        let samples: Vec<PointerSample> = batches
            .iter()
            .flat_map(|batch| batch.pointers[0].samples.clone())
            .collect();
        assert_eq!(samples.len(), 11, "every 4 px over 40 px, both ends");
        assert_eq!(samples[5].position, [20.0, 0.0]);
        assert!((samples[5].pressure - 0.4).abs() < 1e-6);
        assert_eq!(samples[10].time_ms, 1000 + 10 * SAMPLE_MS);
        assert_eq!(batches.first().map(|b| b.phase), Some(MotionPhase::Down));
        assert_eq!(batches.last().map(|b| b.phase), Some(MotionPhase::Up));
        assert!(
            batches
                .iter()
                .all(|b| b.pointers[0].samples.len() <= PER_BATCH)
        );
    }

    #[test]
    fn a_single_point_is_a_dot() {
        let batches = path(Tool::Stylus, &[[5.0, 5.0, 1.0]], 0);
        assert_eq!(phases(&batches), [MotionPhase::Down, MotionPhase::Up]);
    }
}
