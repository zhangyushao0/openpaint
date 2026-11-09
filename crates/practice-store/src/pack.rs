//! A practice pack in one file (ROADMAP S78): its manifest and its pictures, built by the
//! harness (`cargo practice-pack`) and kept as imported beside the library (`packs.rs`). Records
//! with check sums, as an OpenPaint file is (D62): no archive crate, no serialization crate. The
//! pack format is its own number: a pack travels apart from any library.
//!
//! Layout, integers little-endian: the magic `OpenPaintPack\0` (14 bytes); the pack format (u32,
//! `PACK_FORMAT`); the head's length (u32), the head and its CRC-32 (u32, IEEE); then the
//! pictures, each right after the one before in the items' order, the last ending the file. The
//! head: the pack's id and title (u32 length + UTF-8 each); the item count (u32); per item its id
//! (string), kind and tier (u8 each, their codes), the picture's width and height (u32 each), its
//! title (string); its credit, a flag (u8, 1 credited, 0 not: the user's own) and when credited
//! the source page and creator (strings), the licence (u8, its code), attribution and notes
//! (strings); its outline, a contour count (u32, 0 none) and per contour a point count (u32) and
//! each point's x and y in the picture's pixels (f32 each); its value masses (S96), their cells
//! across and down (u32 each, both 0 none) and then a run count (u32) and per run, row by row, a
//! value (u8, dark 0, middle 1, light 2) and how many cells hold it (u32); its colour areas (a
//! crop's, S101), their cells across and down (u32 each, both 0 none), then an area count (u32)
//! and each area's colour (three u8, sRGB), then runs as the masses' of each cell's area (u8);
//! and its picture's length and CRC-32 (u32 each). A picture is a PNG or JPEG file of the item's
//! size.
//!
//! So a pack's list reads its head alone and an exercise one picture at its offset, each
//! checked by its own sum; an import checks every part. Format 4 (1.19.0) is the only one read: a
//! pack of any other is `OtherFormat`.

use std::io::Cursor;
use std::sync::Arc;

use image::{ImageFormat, ImageReader};
use openpaint_app::PortError;
use openpaint_practice::{
    Areas, Credit, GRID, Licence, MASSES_SIDE, Outline, Pack, PackItem, PictureKind, Point, Tier,
    ValueMasses,
};
use openpaint_store::{Reader, Writer};

/// The layout this build reads and writes.
pub const PACK_FORMAT: u32 = 4;
const MAGIC: &[u8; 14] = b"OpenPaintPack\0";
/// Bytes before the head: the magic, the format and the head's length.
pub(crate) const START: usize = MAGIC.len() + 4 + 4;
/// The longest head a pack may have: a length naming more is damage, never an allocation.
const MAX_HEAD: usize = 16 << 20;
/// The fewest bytes of an item in the head (a one-byte id, an empty title, no credit, no
/// outline, no value masses, no colour areas), of a contour, of a point, of a run of a map's
/// cells and of a colour.
const ITEM_BYTES: usize = 5 + 2 + 8 + 4 + 1 + 4 + 8 + 8 + 8;
const CONTOUR_BYTES: usize = 4;
const POINT_BYTES: usize = 8;
const RUN_BYTES: usize = 5;
const COLOUR_BYTES: usize = 3;

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
    head.str(&item.title);
    match &item.credit {
        Some(credit) => {
            head.u8(1);
            head.str(&credit.source);
            head.str(&credit.creator);
            head.u8(credit.licence.code());
            head.str(&credit.attribution);
            head.str(&credit.notes);
        }
        None => head.u8(0),
    }
    let contours = item
        .outline
        .as_ref()
        .map_or(&[][..], |o| o.contours.as_slice());
    head.len(contours.len());
    for contour in contours {
        head.len(contour.len());
        for [x, y] in contour {
            head.f32(*x as f32);
            head.f32(*y as f32);
        }
    }
    match &item.masses {
        Some(masses) => {
            head.u32(masses.size[0]);
            head.u32(masses.size[1]);
            write_runs(head, &masses.values);
        }
        None => {
            head.u32(0);
            head.u32(0);
        }
    }
    let Some(areas) = &item.areas else {
        head.u32(0);
        head.u32(0);
        return;
    };
    head.len(areas.size[0]);
    head.len(areas.size[1]);
    head.len(areas.colours.len());
    for colour in &areas.colours {
        head.bytes(colour);
    }
    write_runs(head, areas.map());
}

/// `values` as runs, their count first: each value (u8) and how many cells in a row hold it (u32).
fn write_runs(head: &mut Writer, values: &[u8]) {
    let runs = runs(values);
    head.len(runs.len());
    for (value, length) in runs {
        head.u8(value);
        head.u32(length);
    }
}

/// `values` as runs: each value and how many cells in a row hold it.
fn runs(values: &[u8]) -> Vec<(u8, u32)> {
    let mut runs: Vec<(u8, u32)> = Vec::new();
    for value in values {
        match runs.last_mut() {
            Some((held, length)) if held == value => *length += 1,
            _ => runs.push((*value, 1)),
        }
    }
    runs
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

/// The pack in a head of this build's format followed by its check sum, and where each item's
/// picture lies in the file.
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
        let mut item = read_item(&mut reader)?;
        item.masses = read_masses(&mut reader)?;
        item.areas = read_areas(&mut reader)?;
        items.push(item);
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
    let title = reader.str()?;
    let credit = match reader.u8()? {
        0 => None,
        1 => Some(Credit {
            source: reader.str()?,
            creator: reader.str()?,
            licence: Licence::of_code(reader.u8()?).ok_or_else(damaged)?,
            attribution: reader.str()?,
            notes: reader.str()?,
        }),
        _ => return Err(damaged()),
    };
    let count = reader.count(CONTOUR_BYTES)?;
    let mut contours = Vec::with_capacity(count);
    for _ in 0..count {
        let points = reader.count(POINT_BYTES)?;
        let mut contour: Vec<Point> = Vec::with_capacity(points);
        for _ in 0..points {
            contour.push([f64::from(reader.f32()?), f64::from(reader.f32()?)]);
        }
        contours.push(contour);
    }
    Ok(PackItem {
        id,
        kind,
        tier,
        size,
        title,
        credit,
        outline: (!contours.is_empty()).then_some(Outline { contours }),
        masses: None,
        areas: None,
    })
}

/// An item's value masses, after its outline (S96); none where both sides are 0.
fn read_masses(reader: &mut Reader<'_>) -> Result<Option<Arc<ValueMasses>>, PortError> {
    let size = [reader.u32()?, reader.u32()?];
    if size == [0, 0] {
        return Ok(None);
    }
    if !size.iter().all(|side| (1..=MASSES_SIDE).contains(side)) {
        return Err(damaged());
    }
    let values = read_runs(reader, size[0] as usize * size[1] as usize)?;
    let masses = ValueMasses::checked(size, values).ok_or_else(damaged)?;
    Ok(Some(Arc::new(masses)))
}

/// An item's colour areas, after its value masses (S101); none where both sides are 0.
fn read_areas(reader: &mut Reader<'_>) -> Result<Option<Arc<Areas>>, PortError> {
    let size = [reader.u32()? as usize, reader.u32()? as usize];
    if size == [0, 0] {
        return Ok(None);
    }
    if !size.iter().all(|side| (1..=GRID).contains(side)) {
        return Err(damaged());
    }
    let count = reader.count(COLOUR_BYTES)?;
    let mut colours = Vec::with_capacity(count);
    for _ in 0..count {
        colours.push([reader.u8()?, reader.u8()?, reader.u8()?]);
    }
    let map = read_runs(reader, size[0] * size[1])?;
    let areas = Areas::of_map(size, map, colours).ok_or_else(damaged)?;
    Ok(Some(Arc::new(areas)))
}

/// A map of `cells` written as runs: never more cells than it holds.
fn read_runs(reader: &mut Reader<'_>, cells: usize) -> Result<Vec<u8>, PortError> {
    let runs = reader.count(RUN_BYTES)?;
    let mut values = Vec::with_capacity(cells);
    for _ in 0..runs {
        let (value, length) = (reader.u8()?, reader.u32()? as usize);
        if values.len() + length > cells {
            return Err(damaged());
        }
        values.extend(std::iter::repeat_n(value, length));
    }
    Ok(values)
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
