//! The storage worker's files (`mod.rs`): an artwork written into the file the user chose or
//! the chosen file added as a new artwork (D62). A mode's files are its own jobs
//! (`practice/files.rs`).

use openpaint_app::{
    FileJob, ImageSource, LibraryAnswer, MAX_ARTWORK_FILE_BYTES, PortError, ProjectStore,
};

/// Runs `job` on `store`; what the gallery is told.
pub(super) fn run(store: &mut dyn ProjectStore, job: FileJob) -> LibraryAnswer {
    match job {
        FileJob::Save { id, sink } => {
            let result = store.artwork_file(id).and_then(|bytes| sink.write(&bytes));
            if let Err(error) = &result {
                log::error!("saving an OpenPaint file failed: {error}");
            }
            LibraryAnswer::FileSaved(result)
        }
        FileJob::Open { source } => {
            let result = read(source, MAX_ARTWORK_FILE_BYTES)
                .and_then(|bytes| store.import_file(&bytes).map(drop));
            if let Err(error) = &result {
                log::warn!("the chosen file was not opened: {error}");
            }
            LibraryAnswer::FileOpened(result)
        }
        // A mode's file goes to the mode (`Runtime::answer`), never here.
        FileJob::Mode { tag, .. } => {
            log::error!("a mode's file (tag {tag}) was handed to the gallery's store");
            LibraryAnswer::FileOpened(Err(PortError::Stopped))
        }
    }
}

/// What `job` answers when the store panicked.
pub(super) fn stopped(job: &FileJob) -> LibraryAnswer {
    let error = PortError::Stopped;
    match job {
        FileJob::Save { .. } => LibraryAnswer::FileSaved(Err(error)),
        FileJob::Open { .. } | FileJob::Mode { .. } => LibraryAnswer::FileOpened(Err(error)),
    }
}

/// `source` read whole, refused when longer than `limit`.
pub(crate) fn read(source: Box<dyn ImageSource>, limit: u64) -> Result<Vec<u8>, PortError> {
    let bytes = source.read(limit)?;
    if bytes.len() as u64 > limit {
        return Err(PortError::Device {
            detail: format!("larger than {limit} bytes"),
        });
    }
    Ok(bytes)
}
