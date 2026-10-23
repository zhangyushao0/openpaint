//! The compare worker: Compare's work off the frame loop, so no Compare stalls a frame. The
//! aligned overlay's fit (S80; S79's group took one 68 ms frame on the tablet), and a painted
//! copy's frame (a colour combination's, a value scale's) drawn from the artwork as the canvas
//! shows it and read (S83: the readback had waited for every tile on the main thread). The open
//! practice sheet hands out one job per Compare; the worker does it and the sheet takes the answer
//! when it is still the one asked for; the overlay or the reading shows once it is there.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use openpaint_app::{CopyRead, FitJob, PictureImage, ReadJob};
use openpaint_core::Artwork;
use openpaint_practice::{Fit, Metrics};
use openpaint_render::Core;

use crate::canvas::{Canvas, PAPER};
use crate::error::RuntimeError;
use crate::shared::SharedCore;
use crate::worker::Queue;

enum Job {
    Fit(FitJob),
    /// The copy's frame of the artwork as it was when Compare asked.
    Read(ReadJob, Artwork),
}

enum Done {
    Fit(u64, Option<Fit>),
    Read(u64, Option<(CopyRead, Metrics)>),
}

pub(crate) struct CompareWorker {
    worker: Queue<Job, Done>,
}

impl CompareWorker {
    pub(crate) fn start(core: SharedCore) -> Result<Self, RuntimeError> {
        let worker = Queue::spawn("openpaint-compare", move |job| match job {
            Job::Fit(job) => fit(&job),
            Job::Read(job, artwork) => read(&core.get(), &job, &artwork),
        })?;
        Ok(Self { worker })
    }

    /// True while a job is queued or running, or its answer waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// Answers finished after their sheet was left: nobody takes them, so they are let go (a
    /// sheet opened again asks anew).
    pub(crate) fn let_go(&mut self) {
        while self.worker.take().is_some() {}
    }

    /// One frame's compare work on `canvas`: answers go to its sheet, new jobs to the worker.
    pub(crate) fn step(&mut self, canvas: &mut Canvas) {
        while let Some(done) = self.worker.take() {
            let session = canvas.session_mut();
            match done {
                Done::Fit(sequence, fit) => session.practice_fitted(sequence, fit),
                Done::Read(sequence, read) => session.practice_read(sequence, read),
            }
        }
        if let Some(job) = canvas.session_mut().practice_fit_request() {
            self.worker.submit(Job::Fit(job));
        }
        if let Some(job) = canvas.session_mut().practice_read_request() {
            let artwork = canvas.session().artwork().clone();
            self.worker.submit(Job::Read(job, artwork));
        }
    }
}

/// The fit, on the worker; a panic is logged and the overlay lies in place.
fn fit(job: &FitJob) -> Done {
    let started = Instant::now();
    let fit = catch_unwind(AssertUnwindSafe(|| job.run())).unwrap_or_else(|_| {
        log::error!("a fit panicked; the overlay lies in place");
        None
    });
    log::info!(
        "fit {} ({}) in {} ms",
        job.sequence,
        if job.shape { "shape" } else { "lines" },
        started.elapsed().as_millis()
    );
    Done::Fit(job.sequence, fit)
}

/// The copy's frame drawn as the canvas shows it (the 1x export is the canvas pixel for pixel)
/// and read, on the worker; a failure is logged and the copy is not read.
fn read(core: &Core, job: &ReadJob, artwork: &Artwork) -> Done {
    let started = Instant::now();
    let drawn = core
        .export_region(artwork, PAPER, (job.origin, job.size))
        .inspect_err(|error| log::error!("drawing a practice copy failed: {error}"));
    let read = drawn.ok().and_then(|image| {
        let pixels = PictureImage {
            size: image.size(),
            pixels: image.into_pixels(),
        };
        catch_unwind(AssertUnwindSafe(|| job.run(&pixels))).unwrap_or_else(|_| {
            log::error!("reading a practice copy panicked");
            None
        })
    });
    log::info!(
        "practice copy {} read in {} ms",
        job.sequence,
        started.elapsed().as_millis()
    );
    Done::Read(job.sequence, read)
}
