//! Practice packs of an older layout upgraded to this build's (`PACK_FORMAT` 4, 1.19.0, S101):
//! format 2 (1.4.0 to 1.13.0) with no value masses and no colour areas, format 3 (1.14.0 to
//! 1.18.1) with no colour areas; the same items with "none" for what their layout lacked, the
//! same pictures, written whole in this build's layout. The only place an older pack is read: a
//! pack file of an older layout is upgraded as it is imported (an official pack left in the
//! Download folder before), and a pack kept beside the library (the personal pack of the user's
//! own pictures among them) the first time the packs are listed (`packs.rs`). Nothing else reads
//! an older layout.

use openpaint_app::PortError;

use crate::codec::Reader;
use crate::pack::{MAGIC, MAX_HEAD, START, damaged, decode_head, encode_pack};

/// The layouts upgraded.
const OLD_FORMATS: [u32; 2] = [2, 3];

/// `bytes` as this build's pack file when they are a pack of an older format; `None` for any
/// other.
///
/// # Errors
/// `Corrupt` for an older pack whose head or pictures are damaged.
pub(crate) fn upgraded(bytes: &[u8]) -> Result<Option<Vec<u8>>, PortError> {
    let Some(rest) = (bytes.first_chunk::<START>()).and_then(|start| start.strip_prefix(MAGIC))
    else {
        return Ok(None);
    };
    let mut reader = Reader::new(rest, "practice pack");
    let format = reader.u32()?;
    if !OLD_FORMATS.contains(&format) {
        return Ok(None);
    }
    let length = reader.u32()? as usize;
    if length > MAX_HEAD {
        return Err(damaged());
    }
    let (head, mut rest) = bytes[START..]
        .split_at_checked(length + 4)
        .ok_or_else(damaged)?;
    let (pack, placed) = decode_head(head, format)?;
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
    log::info!("practice pack {} upgraded from format {format}", pack.id);
    encode_pack(&pack, &pictures).map(Some)
}
