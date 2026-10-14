//! The display cache's worker (S54, DECISIONS D81): writes the open canvas's pixels once the
//! canvas shows a version the library holds complete, and deletes an artwork's cached canvas
//! with the artwork, in the order they were asked for, off the screen's thread: one GPU wait for
//! the copy of the canvas target, LZ4, a file written whole and renamed. A failure is logged and
//! ignored (the cache is derived: the canvas is evaluated instead). Off (`None`) in the harness
//! and the tests: nothing is written or read then.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Instant;

use openpaint_app::ProjectId;
use openpaint_render::{CanvasCopy, Core};
use openpaint_store::{CacheKey, DisplayCache};

use crate::error::RuntimeError;
use crate::worker::Queue;

pub(crate) enum CacheJob {
    /// The canvas of `project`, copied out of its target on `core`, as its cache under `key`.
    Write {
        core: Arc<Core>,
        project: ProjectId,
        key: CacheKey,
        copy: CanvasCopy,
    },
    /// The artwork is deleted: its cached canvas goes too.
    Delete(ProjectId),
}

pub(crate) struct CacheWorker {
    worker: Option<Queue<CacheJob, ()>>,
}

impl CacheWorker {
    /// The worker keeping `cache`; `None`: no display cache.
    pub(crate) fn start(cache: Option<DisplayCache>) -> Result<Self, RuntimeError> {
        let worker = cache
            .map(|cache| Queue::spawn("openpaint-cache", move |job| serve(&cache, job)))
            .transpose()?;
        Ok(Self { worker })
    }

    /// Whether there is a display cache.
    pub(crate) const fn on(&self) -> bool {
        self.worker.is_some()
    }

    pub(crate) fn submit(&mut self, job: CacheJob) {
        if let Some(worker) = &mut self.worker {
            worker.submit(job);
        }
    }

    /// Takes what finished (each frame, before `busy` is asked).
    pub(crate) fn collect(&mut self) {
        if let Some(worker) = &mut self.worker {
            while worker.take().is_some() {}
        }
    }

    /// True while a job is queued or running, or finished but not collected.
    pub(crate) fn busy(&self) -> bool {
        self.worker.as_ref().is_some_and(Queue::busy)
    }
}

fn serve(cache: &DisplayCache, job: CacheJob) {
    let served = catch_unwind(AssertUnwindSafe(|| match job {
        CacheJob::Write {
            core,
            project,
            key,
            copy,
        } => write(cache, (&core, project), &key, copy),
        CacheJob::Delete(project) => match cache.delete(project) {
            Ok(()) => log::info!("display cache of project {} deleted", project.get()),
            Err(error) => log::warn!("the display cache of a deleted artwork stays: {error}"),
        },
    }));
    if served.is_err() {
        log::error!("the display cache's worker panicked");
    }
}

/// Reads `copy` back (the one GPU wait), then writes it as `project`'s cached canvas, timed.
fn write(
    cache: &DisplayCache,
    (core, project): (&Core, ProjectId),
    key: &CacheKey,
    copy: CanvasCopy,
) {
    let started = Instant::now();
    let pixels = match copy.bytes(core) {
        Ok(pixels) => pixels,
        Err(error) => {
            log::warn!("the canvas was not read back for its display cache: {error}");
            return;
        }
    };
    let read = started.elapsed();
    match cache.write(project, key, &pixels) {
        Ok(bytes) => log::info!(
            "display cache of project {} written: {:.1} MB of {:.1} MB in {} ms (readback {} ms), \
             save id {:016x}",
            project.get(),
            bytes as f64 / 1e6,
            pixels.len() as f64 / 1e6,
            started.elapsed().as_millis(),
            read.as_millis(),
            key.saved.get()
        ),
        Err(error) => log::warn!("the display cache was not written: {error}"),
    }
}
