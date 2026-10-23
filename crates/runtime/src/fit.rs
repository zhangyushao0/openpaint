//! The fit worker (S80): Compare's aligned overlay fitted off the frame loop, so no Compare
//! stalls a frame (S79's group took one 68 ms frame on the tablet). The open practice sheet hands
//! out one fit per Compare; the worker finds it and the sheet takes it when it is still the one
//! asked for; the overlay shows once it is there.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use openpaint_app::FitJob;
use openpaint_practice::Fit;

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::worker::Queue;

struct Done {
    sequence: u64,
    fit: Option<Fit>,
}

pub(crate) struct FitWorker {
    worker: Queue<FitJob, Done>,
}

impl FitWorker {
    pub(crate) fn start() -> Result<Self, RuntimeError> {
        let worker = Queue::spawn("openpaint-fit", |job: FitJob| {
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
            Done {
                sequence: job.sequence,
                fit,
            }
        })?;
        Ok(Self { worker })
    }

    /// True while a fit is queued or running, or waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// Fits finished after their sheet was left: nobody takes them, so they are let go (a sheet
    /// opened again asks anew).
    pub(crate) fn let_go(&mut self) {
        while self.worker.take().is_some() {}
    }

    /// One frame's fit work on `canvas`: fits found go to its sheet, a new one to the worker.
    pub(crate) fn step(&mut self, canvas: &mut Canvas) {
        while let Some(done) = self.worker.take() {
            canvas
                .session_mut()
                .practice_fitted(done.sequence, done.fit);
        }
        if let Some(job) = canvas.session_mut().practice_fit_request() {
            self.worker.submit(job);
        }
    }
}
