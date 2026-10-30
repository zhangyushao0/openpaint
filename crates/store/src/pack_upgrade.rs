//! Practice packs of format 2 (1.4.0 to 1.13.0) upgraded to format 3 (1.14.0, S96): the same
//! items with no value masses, the same pictures, written whole in this build's layout. The only
//! place a format-2 pack is read: a pack file of the old layout is upgraded as it is imported (an
//! official pack left in the Download folder before), and a pack kept beside the library (the
//! personal pack of the user's own pictures among them) the first time the packs are listed
//! (`packs.rs`). Nothing else reads the old layout.

use openpaint_app::PortError;

use crate::codec::Reader;
use crate::pack::{MAGIC, MAX_HEAD, START, damaged, decode_head, encode_pack};

/// The layout upgraded.
const OLD_FORMAT: u32 = 2;

/// `bytes` as this build's pack file when they are a pack of format 2; `None` for any other.
///
/// # Errors
/// `Corrupt` for a format-2 pack whose head or pictures are damaged.
pub(crate) fn upgraded(bytes: &[u8]) -> Result<Option<Vec<u8>>, PortError> {
    let Some(rest) = (bytes.first_chunk::<START>()).and_then(|start| start.strip_prefix(MAGIC))
    else {
        return Ok(None);
    };
    let mut reader = Reader::new(rest, "practice pack");
    if reader.u32()? != OLD_FORMAT {
        return Ok(None);
    }
    let length = reader.u32()? as usize;
    if length > MAX_HEAD {
        return Err(damaged());
    }
    let (head, mut rest) = bytes[START..]
        .split_at_checked(length + 4)
        .ok_or_else(damaged)?;
    let (pack, placed) = decode_head(head, OLD_FORMAT)?;
    let mut pictures = Vec::with_capacity(placed.len());
    for place in &placed {
        let (picture, after) = rest
            .split_at_checked(place.length as usize)
            .ok_or_else(damaged)?;
        if crc32fast::hash(picture) != place.check {
            return Err(damaged());
        }
        pictures.push(picture.to_vec());
        rest = after;
    }
    if !rest.is_empty() {
        return Err(damaged());
    }
    log::info!(
        "practice pack {} upgraded from format {OLD_FORMAT}",
        pack.id
    );
    encode_pack(&pack, &pictures).map(Some)
}
