//! Practice's files on the storage worker (`jobs.rs`): a practice pack imported beside the
//! library; a picture of the user's own added to their pack, its silhouette outline derived as
//! the pack builder derives it (shown only when it passes the derivation's checks); and a pack's
//! picture read for the screen.

use openpaint_app::{ImageSource, MAX_IMAGE_BYTES, PortError};
use openpaint_practice::{ItemRef, Pack, Tier, outline};
use openpaint_practice_app::{MAX_PACK_BYTES, PictureImage};
use openpaint_practice_store::{Packs, pack_picture, shown_picture};

use crate::storage::read;

/// The file `source` kept in `packs` as a practice pack (checked whole first).
pub(super) fn import(
    packs: Option<&Packs>,
    source: Box<dyn ImageSource>,
) -> Result<Pack, PortError> {
    let result =
        read(source, MAX_PACK_BYTES).and_then(|bytes| packs.ok_or_else(no_packs)?.import(&bytes));
    if let Err(error) = &result {
        log::warn!("the chosen file was not imported as a practice pack: {error}");
    }
    result
}

/// The picture file `source` added to the user's own pictures at `tier` with the outline derived
/// from it, if it passes; the personal pack as it is now.
pub(super) fn add(
    packs: Option<&Packs>,
    source: Box<dyn ImageSource>,
    tier: Tier,
) -> Result<Pack, PortError> {
    let result = read(source, MAX_IMAGE_BYTES).and_then(|bytes| added(packs, &bytes, tier));
    if let Err(error) = &result {
        log::warn!("the chosen picture was not added: {error}");
    }
    result
}

fn no_packs() -> PortError {
    PortError::Device {
        detail: "no place for practice packs".to_owned(),
    }
}

fn added(packs: Option<&Packs>, bytes: &[u8], tier: Tier) -> Result<Pack, PortError> {
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
