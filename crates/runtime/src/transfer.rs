//! The transfer worker: exports (render through the single evaluation entry, encode, write the
//! file the user chose), reference images (read, decode, scale down) and pictures to insert
//! (read, decode, fit inside the canvas), off the UI thread. It
//! holds only an `Arc<Core>` and the jobs' own values, never anything the frame loop holds; the
//! frame loop takes its results into the app's `Transfers`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use openpaint_app::{
    ContactPhase, ExportError, ExportIntent, ExportJob, ImageError, ImageSource, InsertJob, Intent,
    MAX_IMAGE_BYTES, OpenJob, ReferenceImage, StrokeIntent, TransferJob, Transfers,
};
use openpaint_core::Patch;
use openpaint_render::{Banding, Core, RenderError};
use openpaint_store::{ImageEncoder, open_picture, open_reference};

use crate::canvas::{Canvas, PAPER};
use crate::error::RuntimeError;
use crate::shared::SharedCore;
use crate::worker::Queue;

pub(crate) enum TransferDone {
    Exported(Result<(), ExportError>),
    Opened(Result<ReferenceImage, ImageError>),
    Inserted(Result<Patch, ImageError>),
}

pub(crate) struct TransferWorker {
    worker: Queue<TransferJob, TransferDone>,
}

impl TransferWorker {
    pub(crate) fn start(core: SharedCore) -> Result<Self, RuntimeError> {
        let worker = Queue::spawn("openpaint-transfer", move |job| serve(&core.get(), job))?;
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
                TransferDone::Inserted(result) => transfers.inserted(result),
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
                if let ExportIntent::Start {
                    format,
                    delivery,
                    scale,
                } = export
                {
                    let factor = scale.factor();
                    log::info!("export as {} at {factor}x by {delivery:?}", format.name());
                }
                let open = canvas.map(|canvas| (canvas.session().artwork(), canvas.title()));
                transfers.export(export, open)
            }
            Intent::Reference(reference) => transfers.reference(reference),
            Intent::Insert(insert) => {
                let open = canvas.map(|canvas| canvas.session().artwork().canvas());
                transfers.insert(*insert, open)
            }
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
        TransferJob::Insert(job) => {
            let result = catch_unwind(AssertUnwindSafe(|| insert(job)));
            TransferDone::Inserted(result.unwrap_or_else(|_| {
                log::error!("opening a picture to insert panicked");
                Err(ImageError::Damaged)
            }))
        }
    }
}

/// Renders the job's artwork at its scale in bands, streams each band into the encoder (S29: a
/// 4x export is never held whole as pixels) and writes the chosen file.
fn export(core: &Core, job: ExportJob) -> Result<(), ExportError> {
    let started = Instant::now();
    let rendering = |error: RenderError| {
        log::error!("export rendering failed: {error}");
        ExportError::Render
    };
    let scale = job.scale.factor();
    let mut bands = core
        .export_bands(&job.artwork, PAPER, scale, Banding::DEFAULT)
        .map_err(rendering)?;
    let size = bands.size();
    let mut encoder = ImageEncoder::new(job.format, size)?;
    let mut encoding = Duration::ZERO;
    for band in &mut bands {
        let band = band.map_err(rendering)?;
        let at = Instant::now();
        encoder.rows(band.pixels())?;
        encoding += at.elapsed();
    }
    let (planned, waited) = bands.timing();
    let at = Instant::now();
    let bytes = encoder.finish()?;
    encoding += at.elapsed();
    let written = Instant::now();
    job.sink
        .write(&bytes)
        .inspect_err(|error| log::error!("writing the export failed: {error}"))
        .map_err(ExportError::Write)?;
    let [width, height] = size;
    log::info!(
        "exported {} at {scale}x, {width} x {height}, {} bytes in {} ms: planned and submitted \
         {} ms, GPU and readback {} ms, encoded {} ms, written {} ms",
        job.format.name(),
        bytes.len(),
        started.elapsed().as_millis(),
        planned.as_millis(),
        waited.as_millis(),
        encoding.as_millis(),
        written.elapsed().as_millis()
    );
    Ok(())
}

/// Reads the chosen file (at most `MAX_IMAGE_BYTES`) and decodes it as a reference image.
fn open(job: OpenJob) -> Result<ReferenceImage, ImageError> {
    let started = Instant::now();
    let bytes = read(job.source)?;
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

/// Reads the chosen file (at most `MAX_IMAGE_BYTES`) and fits its picture inside the canvas.
fn insert(job: InsertJob) -> Result<Patch, ImageError> {
    let started = Instant::now();
    let bytes = read(job.source)?;
    let picture = open_picture(&bytes, job.canvas)
        .inspect_err(|error| log::warn!("the chosen picture cannot be inserted: {error}"))?;
    let ([x, y], [width, height]) = (picture.origin(), picture.size());
    log::info!(
        "picture to insert opened at {width} x {height} (at {x}, {y}) from {} bytes in {} ms",
        bytes.len(),
        started.elapsed().as_millis()
    );
    Ok(picture)
}

fn read(source: Box<dyn ImageSource>) -> Result<Vec<u8>, ImageError> {
    let bytes = source.read(MAX_IMAGE_BYTES).map_err(ImageError::Read)?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(ImageError::TooLarge);
    }
    Ok(bytes)
}
