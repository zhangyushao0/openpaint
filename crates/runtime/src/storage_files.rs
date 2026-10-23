//! The storage worker's files (`storage.rs`): an artwork written into the file the user chose or
//! the chosen file added as a new artwork (D62); a practice pack imported beside the library; a
//! picture of the user's own added to their pack, its silhouette outline derived as the pack
//! builder derives it (shown only when it passes the derivation's checks); and a practice
//! picture read for the screen.

use openpaint_app::{
    FileJob, ImageSource, LibraryAnswer, MAX_ARTWORK_FILE_BYTES, MAX_IMAGE_BYTES, MAX_PACK_BYTES,
    PictureImage, PortError, ProjectStore,
};
use openpaint_practice::{ItemRef, Pack, Tier, outline};
use openpaint_store::{Packs, pack_picture, shown_picture};

/// Runs `job` on `store` and `packs`; what the gallery is told.
pub(super) fn run(
    store: &mut dyn ProjectStore,
    packs: Option<&Packs>,
    job: FileJob,
) -> LibraryAnswer {
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
        FileJob::ImportPack { source } => {
            let result = read(source, MAX_PACK_BYTES)
                .and_then(|bytes| packs.ok_or_else(no_packs)?.import(&bytes));
            if let Err(error) = &result {
                log::warn!("the chosen file was not imported as a practice pack: {error}");
            }
            LibraryAnswer::PackImported(result)
        }
        FileJob::AddPicture { source, tier } => {
            let result = read(source, MAX_IMAGE_BYTES).and_then(|bytes| add(packs, &bytes, tier));
            if let Err(error) = &result {
                log::warn!("the chosen picture was not added: {error}");
            }
            LibraryAnswer::PictureAdded(result)
        }
    }
}

/// What `job` answers when the store panicked.
pub(super) fn stopped(job: &FileJob) -> LibraryAnswer {
    let error = PortError::Stopped;
    match job {
        FileJob::Save { .. } => LibraryAnswer::FileSaved(Err(error)),
        FileJob::Open { .. } => LibraryAnswer::FileOpened(Err(error)),
        FileJob::ImportPack { .. } => LibraryAnswer::PackImported(Err(error)),
        FileJob::AddPicture { .. } => LibraryAnswer::PictureAdded(Err(error)),
    }
}

/// `source` read whole, refused when longer than `limit`.
fn read(source: Box<dyn ImageSource>, limit: u64) -> Result<Vec<u8>, PortError> {
    let bytes = source.read(limit)?;
    if bytes.len() as u64 > limit {
        return Err(PortError::Device {
            detail: format!("larger than {limit} bytes"),
        });
    }
    Ok(bytes)
}

fn no_packs() -> PortError {
    PortError::Device {
        detail: "no place for practice packs".to_owned(),
    }
}

/// The picture file `bytes` added to the user's own pictures at `tier` with the outline derived
/// from it, if it passes; the personal pack as it is now.
fn add(packs: Option<&Packs>, bytes: &[u8], tier: Tier) -> Result<Pack, PortError> {
    let packs = packs.ok_or_else(no_packs)?;
    let picture = pack_picture(bytes).map_err(|error| {
        log::warn!("the chosen picture is not taken: {error}");
        PortError::Corrupt { what: "picture" }
    })?;
    let derived = outline(picture.size, &picture.rgba)
        .inspect_err(|refusal| log::info!("the chosen picture has no outline: {refusal}"))
        .ok();
    packs.add_picture(&picture, (derived, tier))
}

/// `item`'s picture read and reduced for the screen; none when it cannot be (logged).
pub(super) fn picture(packs: Option<&Packs>, item: &ItemRef) -> Option<PictureImage> {
    let file = match packs.map(|packs| packs.picture(item)) {
        Some(Ok(Some(file))) => file,
        Some(Ok(None)) | None => {
            log::warn!("no pack kept holds the picture {}/{}", item.pack, item.item);
            return None;
        }
        Some(Err(error)) => {
            log::warn!(
                "the picture {}/{} is not read: {error}",
                item.pack,
                item.item
            );
            return None;
        }
    };
    shown_picture(&file)
        .inspect_err(|error| {
            log::warn!(
                "the picture {}/{} does not decode: {error}",
                item.pack,
                item.item
            )
        })
        .ok()
}
