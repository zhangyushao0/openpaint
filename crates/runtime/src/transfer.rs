//! The transfer worker: exports (render through the single evaluation entry, encode, write the
//! file the user chose) and reference images (read, decode, scale down), off the UI thread. It
//! holds only an `Arc<Core>` and the jobs' own values, never anything the frame loop holds; the
//! frame loop takes its results into the app's `Transfers`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Instant;

use openpaint_app::{
    ContactPhase, ExportError, ExportIntent, ExportJob, ImageError, Intent, MAX_IMAGE_BYTES,
    OpenJob, ReferenceImage, StrokeIntent, TransferJob, Transfers,
};
use openpaint_render::Core;
use openpaint_store::{encode_image, open_reference};

use crate::canvas::{Canvas, PAPER};
use crate::error::RuntimeError;
use crate::worker::Queue;

pub(crate) enum TransferDone {
    Exported(Result<(), ExportError>),
    Opened(Result<ReferenceImage, ImageError>),
}

pub(crate) struct TransferWorker {
    worker: Queue<TransferJob, TransferDone>,
}

impl TransferWorker {
    pub(crate) fn start(core: Arc<Core>) -> Result<Self, RuntimeError> {
        let worker = Queue::spawn("openpaint-transfer", move |job| serve(&core, job))?;
        Ok(Self { worker })
    }

    pub(crate) fn submit(&mut self, job: TransferJob) {
        self.worker.submit(job);
    }

    /// True while a job is queued or running, or its result waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// Hands every finished job's result to `transfers`.
    pub(crate) fn receive(&mut self, transfers: &mut Transfers) {
        while let Some(done) = self.worker.take() {
            match done {
                TransferDone::Exported(result) => transfers.exported(result),
                TransferDone::Opened(result) => transfers.opened(result),
            }
        }
    }
}

/// Applies the frame's export and reference intents to `transfers` and returns the others in
/// order. An export takes the version on the open `canvas`. A contact landing on the canvas
/// lets a success notice go.
pub(crate) fn route(
    transfers: &mut Transfers,
    canvas: Option<&Canvas>,
    intents: Vec<Intent>,
) -> Vec<Intent> {
    let mut rest = Vec::with_capacity(intents.len());
    for intent in intents {
        let result = match &intent {
            Intent::Export(export) => {
                if let ExportIntent::Start { format, delivery } = export {
                    log::info!("export as {} by {delivery:?}", format.name());
                }
                let open = canvas.map(|canvas| (canvas.session().artwork(), canvas.title()));
                transfers.export(export, open)
            }
            Intent::Reference(reference) => transfers.reference(reference),
            Intent::Stroke(StrokeIntent::Contact(contact)) => {
                if contact.phase == ContactPhase::Down {
                    transfers.interrupt();
                }
                rest.push(intent);
                Ok(())
            }
            _ => {
                rest.push(intent);
                Ok(())
            }
        };
        if let Err(error) = result {
            log::warn!("intent refused: {error}");
        }
    }
    rest
}

/// One job. A panic is a bug: it is logged and reported as a failure of that job.
fn serve(core: &Core, job: TransferJob) -> TransferDone {
    match job {
        TransferJob::Export(job) => {
            let result = catch_unwind(AssertUnwindSafe(|| export(core, job)));
            TransferDone::Exported(result.unwrap_or_else(|_| {
                log::error!("exporting panicked");
                Err(ExportError::Render)
            }))
        }
        TransferJob::Open(job) => {
            let result = catch_unwind(AssertUnwindSafe(|| open(job)));
            TransferDone::Opened(result.unwrap_or_else(|_| {
                log::error!("opening an image panicked");
                Err(ImageError::Damaged)
            }))
        }
    }
}

/// Renders the job's artwork at its full size, encodes it and writes the chosen file.
fn export(core: &Core, job: ExportJob) -> Result<(), ExportError> {
    let started = Instant::now();
    let image = core
        .export(&job.artwork, PAPER)
        .inspect_err(|error| log::error!("export rendering failed: {error}"))
        .map_err(|_| ExportError::Render)?;
    let rendered = started.elapsed();
    let bytes = encode_image(job.format, image.size(), image.pixels())?;
    job.sink
        .write(&bytes)
        .inspect_err(|error| log::error!("writing the export failed: {error}"))
        .map_err(ExportError::Write)?;
    let [width, height] = image.size();
    log::info!(
        "exported {} {width} x {height}, {} bytes: rendered in {} ms, encoded and written in {} ms",
        job.format.name(),
        bytes.len(),
        rendered.as_millis(),
        (started.elapsed() - rendered).as_millis()
    );
    Ok(())
}

/// Reads the chosen file (at most `MAX_IMAGE_BYTES`) and decodes it as a reference image.
fn open(job: OpenJob) -> Result<ReferenceImage, ImageError> {
    let started = Instant::now();
    let bytes = job.source.read(MAX_IMAGE_BYTES).map_err(ImageError::Read)?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(ImageError::TooLarge);
    }
    let image = open_reference(&bytes)
        .inspect_err(|error| log::warn!("the chosen image cannot be used: {error}"))?;
    let [width, height] = image.size();
    log::info!(
        "reference image opened at {width} x {height} from {} bytes in {} ms",
        bytes.len(),
        started.elapsed().as_millis()
    );
    Ok(image)
}
