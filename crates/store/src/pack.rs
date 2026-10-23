//! A practice pack in one file (ROADMAP S78): its manifest and its pictures, built by the
//! harness (`cargo practice-pack`) and kept as imported beside the library (`packs.rs`). Records
//! with check sums, as an OpenPaint file is (D62): no archive crate, no serialization crate. The
//! pack format is its own number: a pack travels apart from any library.
//!
//! Layout, integers little-endian: the magic `OpenPaintPack\0` (14 bytes); the pack format (u32,
//! `PACK_FORMAT`); the head's length (u32), the head and its CRC-32 (u32, IEEE); then the
//! pictures, each right after the one before in the items' order, the last ending the file. The
//! head: the pack's id and title (u32 length + UTF-8 each); the item count (u32); per item its id
//! (string), kind and tier (u8 each, their codes), the picture's width and height (u32 each), the
//! source page, title and creator (strings), the licence (u8, its code), attribution and notes
//! (strings), and its picture's length and CRC-32 (u32 each). A picture is a PNG or JPEG file of
//! the item's size.
//!
//! So a pack's list reads its head alone and an exercise one picture at its offset, each
//! checked by its own sum; an import checks every part.

use std::io::Cursor;

use image::{ImageFormat, ImageReader};
use openpaint_app::PortError;
use openpaint_practice::{Licence, Pack, PackItem, PictureKind, Tier};

use crate::codec::{Reader, Writer};

/// The layout this build reads and writes.
pub const PACK_FORMAT: u32 = 1;
const MAGIC: &[u8; 14] = b"OpenPaintPack\0";
/// Bytes before the head: the magic, the format and the head's length.
pub(crate) const START: usize = MAGIC.len() + 4 + 4;
/// The longest head a pack may have: a length naming more is damage, never an allocation.
const MAX_HEAD: usize = 16 << 20;
/// The fewest bytes of an item in the head (empty strings, a one-byte id).
const ITEM_BYTES: usize = 5 + 2 + 8 + 3 * 4 + 1 + 2 * 4 + 8;

/// Where an item's picture lies in its pack's file.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Placed {
    pub(crate) offset: u64,
    pub(crate) length: u32,
    pub(crate) check: u32,
}

pub(crate) const fn damaged() -> PortError {
    PortError::Corrupt {
        what: "practice pack",
    }
}

/// `pack` and its pictures (each a PNG or JPEG of its item's size, in the items' order) as a
/// pack file.
///
/// # Errors
/// `Corrupt` when the pack breaks a rule of `Pack::check` (logged with the item) or a picture is
/// missing, not a PNG or JPEG, or not of its item's size.
pub fn encode_pack(pack: &Pack, pictures: &[Vec<u8>]) -> Result<Vec<u8>, PortError> {
    pack.check().map_err(|problem| {
        log::warn!("a practice pack is refused: {problem}");
        damaged()
    })?;
    if pictures.len() != pack.items.len() {
        return Err(damaged());
    }
    let mut head = Writer::with_capacity(256 * pack.items.len());
    head.str(&pack.id);
    head.str(&pack.title);
    head.len(pack.items.len());
    for (item, picture) in pack.items.iter().zip(pictures) {
        if picture_size(picture) != Some(item.size) {
            log::warn!("a practice pack is refused: item {}'s picture", item.id);
            return Err(damaged());
        }
        write_item(&mut head, item);
        head.len(picture.len());
        head.u32(crc32fast::hash(picture));
    }
    let head = head.finish();
    let pictures_bytes: usize = pictures.iter().map(Vec::len).sum();
    let mut file = Writer::with_capacity(START + head.len() + 4 + pictures_bytes);
    file.bytes(MAGIC);
    file.u32(PACK_FORMAT);
    file.len(head.len());
    file.bytes(&head);
    file.u32(crc32fast::hash(&head));
    for picture in pictures {
        file.bytes(picture);
    }
    Ok(file.finish())
}

fn write_item(head: &mut Writer, item: &PackItem) {
    head.str(&item.id);
    head.u8(item.kind.code());
    head.u8(item.tier.code());
    head.u32(item.size[0]);
    head.u32(item.size[1]);
    head.str(&item.source);
    head.str(&item.title);
    head.str(&item.creator);
    head.u8(item.licence.code());
    head.str(&item.attribution);
    head.str(&item.notes);
}

/// Opens a pack file and checks every part: its head and every picture against their sums, the
/// pack's rules, each picture a PNG or JPEG of its item's size, nothing after the last.
///
/// # Errors
/// `Corrupt` for anything but a whole pack, `OtherFormat` for a pack of another format.
pub fn open_pack(bytes: &[u8]) -> Result<(Pack, Vec<&[u8]>), PortError> {
    let start = bytes.first_chunk::<START>().ok_or_else(damaged)?;
    let length = head_length(start)?;
    let (head, mut rest) = bytes[START..]
        .split_at_checked(length + 4)
        .ok_or_else(damaged)?;
    let (pack, placed) = decode_head(head)?;
    let mut pictures = Vec::with_capacity(placed.len());
    for (item, place) in pack.items.iter().zip(&placed) {
        let (picture, after) = rest
            .split_at_checked(place.length as usize)
            .ok_or_else(damaged)?;
        if crc32fast::hash(picture) != place.check || picture_size(picture) != Some(item.size) {
            return Err(damaged());
        }
        pictures.push(picture);
        rest = after;
    }
    if !rest.is_empty() {
        return Err(damaged());
    }
    Ok((pack, pictures))
}

/// The length of the head of a pack file starting with `start`.
///
/// # Errors
/// `Corrupt` without the magic or for a head longer than any pack's, `OtherFormat` for another
/// format.
pub(crate) fn head_length(start: &[u8; START]) -> Result<usize, PortError> {
    let rest = start.strip_prefix(MAGIC).ok_or_else(damaged)?;
    let mut reader = Reader::new(rest, "practice pack");
    let format = reader.u32()?;
    if format != PACK_FORMAT {
        return Err(PortError::OtherFormat {
            found: Some(format),
        });
    }
    let length = reader.u32()? as usize;
    if length > MAX_HEAD {
        return Err(damaged());
    }
    Ok(length)
}

/// The pack in a head followed by its check sum, and where each item's picture lies in the file.
///
/// # Errors
/// `Corrupt` when the sum does not hold, the head does not decode or the pack breaks a rule.
pub(crate) fn decode_head(head_and_check: &[u8]) -> Result<(Pack, Vec<Placed>), PortError> {
    let (head, check) = head_and_check.split_last_chunk::<4>().ok_or_else(damaged)?;
    if crc32fast::hash(head) != u32::from_le_bytes(*check) {
        return Err(damaged());
    }
    let mut reader = Reader::new(head, "practice pack");
    let (id, title) = (reader.str()?, reader.str()?);
    let count = reader.count(ITEM_BYTES)?;
    let mut items = Vec::with_capacity(count);
    let mut placed = Vec::with_capacity(count);
    let mut offset = (START + head_and_check.len()) as u64;
    for _ in 0..count {
        items.push(read_item(&mut reader)?);
        let (length, check) = (reader.u32()?, reader.u32()?);
        placed.push(Placed {
            offset,
            length,
            check,
        });
        offset += u64::from(length);
    }
    reader.end()?;
    let pack = Pack { id, title, items };
    pack.check().map_err(|problem| {
        log::warn!("a practice pack is not read: {problem}");
        damaged()
    })?;
    Ok((pack, placed))
}

fn read_item(reader: &mut Reader<'_>) -> Result<PackItem, PortError> {
    let id = reader.str()?;
    let kind = PictureKind::of_code(reader.u8()?).ok_or_else(damaged)?;
    let tier = Tier::of_code(reader.u8()?).ok_or_else(damaged)?;
    let size = [reader.u32()?, reader.u32()?];
    let (source, title, creator) = (reader.str()?, reader.str()?, reader.str()?);
    let licence = Licence::of_code(reader.u8()?).ok_or_else(damaged)?;
    let (attribution, notes) = (reader.str()?, reader.str()?);
    Ok(PackItem {
        id,
        kind,
        tier,
        size,
        source,
        title,
        creator,
        licence,
        attribution,
        notes,
    })
}

/// The width and height a PNG or JPEG file's header states; `None` for any other bytes.
pub(crate) fn picture_size(bytes: &[u8]) -> Option<[u32; 2]> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    if !matches!(reader.format(), Some(ImageFormat::Png | ImageFormat::Jpeg)) {
        return None;
    }
    let (width, height) = reader.into_dimensions().ok()?;
    Some([width, height])
}
