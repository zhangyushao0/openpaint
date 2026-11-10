//! The fill worker: samples the active layer through the renderer (a target of its own) and
//! floods it on the CPU (`openpaint-raster`), off the UI thread; a job naming a box (its mode's
//! clip's) is flooded within that box only.
//! While one fill is adjusted it keeps that layer's pixels, so each new threshold costs only the
//! flood; they go when the fill ends. One job runs at a time and the session hands out only its
//! newest request, so a moving hand never queues work.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use openpaint_app::{FillFailed, FillJob};
use openpaint_core::{Artwork, CanvasSize, LayerId, RegionMask};
use openpaint_raster::{FillError, connected_region};
use openpaint_render::{Core, RgbaImage};

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::shared::SharedCore;
use crate::worker::Queue;

enum Task {
    Fill(FillJob),
    /// Lets the kept layer pixels go.
    Release,
}

struct Done {
    sequence: u64,
    result: Result<RegionMask, FillFailed>,
}

/// The layer pixels of the fill being adjusted.
struct Source {
    artwork: Artwork,
    layer: LayerId,
    pixels: RgbaImage,
}

pub(crate) struct FillWorker {
    worker: Queue<Task, Option<Done>>,
    /// The worker may still keep layer pixels.
    holding: bool,
}

impl FillWorker {
    pub(crate) fn start(core: SharedCore) -> Result<Self, RuntimeError> {
        let mut source = None;
        let worker = Queue::spawn("openpaint-fill", move |task| {
            serve(&core.get(), &mut source, task)
        })?;
        Ok(Self {
            worker,
            holding: false,
        })
    }

    /// True while a job is queued or running, or its result waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// One frame's fill work on `canvas`: finished fills go to its session, its newest request
    /// to the worker; once no fill is being placed the kept pixels are let go.
    pub(crate) fn step(&mut self, canvas: &mut Canvas) {
        while let Some(done) = self.worker.take() {
            if let Some(done) = done {
                canvas.fill_finished(done.sequence, done.result);
            }
        }
        if let Some(job) = canvas.session_mut().fill_request() {
            self.worker.submit(Task::Fill(job));
            self.holding = true;
        } else if self.holding && !canvas.session().filling() && !self.worker.busy() {
            self.worker.submit(Task::Release);
            self.holding = false;
        }
    }
}

fn serve(core: &Core, source: &mut Option<Source>, task: Task) -> Option<Done> {
    match task {
        Task::Release => {
            *source = None;
            None
        }
        Task::Fill(job) => {
            let sequence = job.sequence;
            let result = catch_unwind(AssertUnwindSafe(|| fill(core, source, &job)));
            Some(Done {
                sequence,
                result: result.unwrap_or_else(|_| {
                    log::error!("filling panicked");
                    *source = None;
                    Err(FillFailed)
                }),
            })
        }
    }
}

/// Samples the job's layer unless it is the one kept, then floods it.
fn fill(core: &Core, source: &mut Option<Source>, job: &FillJob) -> Result<RegionMask, FillFailed> {
    let started = Instant::now();
    let kept = source
        .as_ref()
        .is_some_and(|s| s.artwork.same_version(&job.artwork) && s.layer == job.layer);
    if !kept {
        *source = None;
        // On a mask the flood runs over its grey (the version tells the two apart).
        let pixels = if job.mask {
            core.mask_pixels(&job.artwork, job.layer)
        } else {
            core.layer_pixels(&job.artwork, job.layer)
        };
        let pixels = pixels
            .inspect_err(|error| log::error!("sampling the layer for a fill failed: {error}"))
            .map_err(|_| FillFailed)?;
        *source = Some(Source {
            artwork: job.artwork.clone(),
            layer: job.layer,
            pixels,
        });
    }
    let sampled = started.elapsed();
    let pixels = source.as_ref().map(|s| &s.pixels).ok_or(FillFailed)?;
    let canvas = job.artwork.canvas();
    let mask = flood(
        canvas,
        pixels.pixels(),
        job.within,
        job.point,
        job.threshold,
    )
    .inspect_err(|error| log::error!("no fill region: {error}"))
    .map_err(|_| FillFailed)?;
    log::info!(
        "fill {} at {:?}, threshold {:.3}: sampled in {} ms{}, flooded in {} ms, {:?} px box",
        job.sequence,
        job.point,
        job.threshold,
        sampled.as_millis(),
        if kept { " (kept)" } else { "" },
        (started.elapsed() - sampled).as_millis(),
        mask.size()
    );
    Ok(mask)
}

/// The region flooded from `point` at `threshold` over `pixels` (the whole `canvas`), found
/// within the box `within` (`(origin, size)`) when one is given: the box's pixels are cropped,
/// flooded, and the region placed back on the canvas.
fn flood(
    canvas: CanvasSize,
    pixels: &[u8],
    within: Option<([u32; 2], [u32; 2])>,
    point: [u32; 2],
    threshold: f32,
) -> Result<RegionMask, FillError> {
    let Some((origin, size)) = within else {
        return connected_region(canvas, pixels, point, threshold);
    };
    let crop = CanvasSize::new(size[0], size[1]).map_err(|_| FillError::InvalidInput)?;
    let seed = [
        point[0].checked_sub(origin[0]),
        point[1].checked_sub(origin[1]),
    ];
    let across = origin[0].checked_add(size[0]);
    let ([Some(x), Some(y)], true) = (seed, across.is_some_and(|end| end <= canvas.width())) else {
        return Err(FillError::InvalidInput);
    };
    let (row, stride) = (size[0] as usize * 4, canvas.width() as usize * 4);
    let mut cropped = Vec::with_capacity(row * size[1] as usize);
    for line in origin[1]..origin[1].saturating_add(size[1]) {
        let start = line as usize * stride + origin[0] as usize * 4;
        let pixels = pixels.get(start..start + row);
        cropped.extend_from_slice(pixels.ok_or(FillError::InvalidInput)?);
    }
    let region = connected_region(crop, &cropped, [x, y], threshold)?;
    let at = [
        origin[0] + region.origin()[0],
        origin[1] + region.origin()[1],
    ];
    RegionMask::from_runs(canvas, at, region.size(), &region.runs())
        .map_err(|_| FillError::InvalidInput)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paper 40 x 20 with a frame's box at (10, 5) of 20 x 10, crossed edge to edge by a black
    /// column at x = 20; nothing marks the box itself, as on a practice sheet.
    fn crossed() -> (CanvasSize, Vec<u8>) {
        let canvas = CanvasSize::new(40, 20).expect("size");
        let mut pixels = vec![0u8; 40 * 20 * 4];
        for y in 5..15 {
            let at = (y * 40 + 20) * 4;
            pixels[at..at + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
        (canvas, pixels)
    }

    const BOX: ([u32; 2], [u32; 2]) = ([10, 5], [20, 10]);

    #[test]
    fn a_flood_in_a_box_stays_on_its_side_of_a_line_across_it() {
        let (canvas, pixels) = crossed();
        let whole = flood(canvas, &pixels, None, [12, 8], 0.6).expect("region");
        assert!(whole.contains([25, 8]), "round the line through the paper");
        assert_eq!(
            whole,
            connected_region(canvas, &pixels, [12, 8], 0.6).expect("region"),
            "without a box the flood is the whole canvas's"
        );
        let boxed = flood(canvas, &pixels, Some(BOX), [12, 8], 0.6).expect("region");
        assert!(boxed.contains([12, 8]) && boxed.contains([10, 14]));
        assert!(
            !boxed.contains([21, 8]) && !boxed.contains([25, 8]),
            "one side only"
        );
        assert!(
            !boxed.contains([5, 8]) && !boxed.contains([12, 2]),
            "inside the box"
        );
        assert_eq!(boxed.origin(), [10, 5]);
        let other = flood(canvas, &pixels, Some(BOX), [25, 8], 0.6).expect("region");
        assert!(other.contains([29, 14]) && !other.contains([12, 8]));
    }

    #[test]
    fn a_box_off_the_canvas_or_a_point_outside_it_gives_no_region() {
        let (canvas, pixels) = crossed();
        let wide = Some(([30, 0], [20, 5]));
        assert!(flood(canvas, &pixels, wide, [35, 2], 0.6).is_err());
        assert!(flood(canvas, &pixels, Some(BOX), [2, 2], 0.6).is_err());
    }
}
