//! The fill worker: samples the active layer through the renderer (its own target, the single
//! evaluator) and floods it on the CPU (`openpaint-raster`), off the UI thread. While one fill
//! is adjusted it keeps that layer's pixels, so each new threshold costs only the flood; they
//! go when the fill ends. One job runs at a time and the session hands out only its newest
//! request, so a moving hand never queues work.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use openpaint_app::{FillFailed, FillJob};
use openpaint_core::{Artwork, LayerId, RegionMask};
use openpaint_raster::connected_region;
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
    let mask = connected_region(canvas, pixels.pixels(), job.point, job.threshold)
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
