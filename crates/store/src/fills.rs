//! Region edit records (format 1, `records.rs` has the keys): one per fill, cleared pixels,
//! transform, adjustment or merged layer, keyed like strokes, `[project u64]['e'][slot
//! u64][index u64]`, oldest at index 0. A value starts with its kind (u8: 0 fill, 1 cleared
//! pixels, 2 transform, 3 adjustment, `adjusts.rs`, 4 a merged layer, `nested.rs`) and how many
//! of its slot's strokes lie under it (u32).
//!
//! - A fill: red, green, blue (f32), alpha locked (u8), its mask (`write_mask`).
//! - Cleared pixels (a cut, or what a copy leaves outside its selection): the mask.
//! - A transform: what moved (u8: 0 the whole layer, 1 a mask), its placement (a, b, c, d, then
//!   the translation: f32), then its pixels (u8: 0 none, a record: the content is the layer's own;
//!   1 a patch, an inserted picture or the drag's lift: its rectangle (x, y, width, height: u32)
//!   and its premultiplied RGBA as a PNG file to the record's end).

use std::io::Cursor;

use image::codecs::png::{CompressionType, FilterType, PngDecoder, PngEncoder};
use image::{ExtendedColorType, ImageDecoder, ImageEncoder};
use openpaint_app::{PortError, ProjectId};
use openpaint_core::{
    ClearParts, EditParts, FillParts, MaskParts, PatchParts, RegionEdit, TransformEdit,
    TransformParts,
};

use crate::adjusts::{KIND_ADJUST, encode_adjust, read_adjust};
use crate::codec::{Reader, Writer};
use crate::nested::{Counts, KIND_NESTED, encode_nested, read_nested};
use crate::records::project_prefix;

const EDIT: u8 = b'e';
const KIND_FILL: u8 = 0;
const KIND_CLEAR: u8 = 1;
const KIND_TRANSFORM: u8 = 2;

/// The prefix of every edit record of `project`.
pub(crate) fn fills_prefix(project: ProjectId) -> Vec<u8> {
    let mut key = project_prefix(project);
    key.push(EDIT);
    key
}

/// The prefix of the edit records of one slot, in order.
pub(crate) fn fill_layer_prefix(project: ProjectId, slot: u64) -> Vec<u8> {
    let mut key = fills_prefix(project);
    key.extend_from_slice(&slot.to_be_bytes());
    key
}

pub(crate) fn fill_key(project: ProjectId, slot: u64, index: usize) -> Vec<u8> {
    let mut key = fill_layer_prefix(project, slot);
    key.extend_from_slice(&(index as u64).to_be_bytes());
    key
}

/// A record's writer holding its kind and how many strokes lie under it.
pub(crate) fn edit_writer(kind: u8, after: usize, capacity: usize) -> Writer {
    let mut writer = Writer::with_capacity(capacity);
    writer.u8(kind);
    writer.len(after);
    writer
}

pub(crate) fn encode_edit(edit: &RegionEdit) -> Vec<u8> {
    match edit {
        RegionEdit::Fill(fill) => {
            let mask = fill.mask().to_parts();
            let mut writer = edit_writer(KIND_FILL, fill.after(), 48 + mask.runs.len() * 2);
            let color = fill.color();
            for channel in [color.r(), color.g(), color.b()] {
                writer.f32(channel);
            }
            writer.bool(fill.alpha_locked());
            write_mask(&mut writer, &mask);
            writer.finish()
        }
        RegionEdit::Clear(clear) => {
            let mask = clear.mask().to_parts();
            let mut writer = edit_writer(KIND_CLEAR, clear.after(), 32 + mask.runs.len() * 2);
            write_mask(&mut writer, &mask);
            writer.finish()
        }
        RegionEdit::Transform(transform) => encode_transform(transform),
        RegionEdit::Adjust(adjust) => encode_adjust(adjust),
        RegionEdit::Nested(nested) => encode_nested(nested),
    }
}

fn encode_transform(transform: &TransformEdit) -> Vec<u8> {
    let mut writer = edit_writer(KIND_TRANSFORM, transform.after(), 96);
    match transform.lift() {
        Some(lift) => {
            writer.u8(1);
            write_mask(&mut writer, &lift.to_parts());
        }
        None => writer.u8(0),
    }
    let placement = transform.placement();
    for value in placement
        .matrix()
        .into_iter()
        .chain(placement.translation())
    {
        writer.f32(value);
    }
    match transform.patch() {
        Some(patch) => {
            writer.u8(1);
            for value in patch.origin().into_iter().chain(patch.size()) {
                writer.u32(value);
            }
            writer.bytes(&encode_pixels(patch.size(), patch.pixels()));
        }
        None => writer.u8(0),
    }
    writer.finish()
}

/// An edit record as plain data (a nested record's content left empty).
#[cfg(test)]
pub(crate) fn decode_edit(bytes: &[u8]) -> Result<EditParts, PortError> {
    decode_record(bytes).map(|(edit, _)| edit)
}

/// An edit record as plain data and, for a nested record, how many records its content holds
/// (`read.rs` reads them).
pub(crate) fn decode_record(bytes: &[u8]) -> Result<(EditParts, Option<Counts>), PortError> {
    let mut reader = Reader::new(bytes, "edit");
    let kind = reader.u8()?;
    let after = u64::from(reader.u32()?);
    let edit = match kind {
        KIND_FILL => EditParts::Fill(read_fill(&mut reader, after)?),
        KIND_CLEAR => EditParts::Clear(ClearParts {
            after,
            mask: read_mask(&mut reader)?,
        }),
        KIND_TRANSFORM => EditParts::Transform(read_transform(&mut reader, after)?),
        KIND_ADJUST => EditParts::Adjust(read_adjust(&mut reader, after)?),
        KIND_NESTED => {
            let (nested, counts) = read_nested(&mut reader, after)?;
            reader.end()?;
            return Ok((EditParts::Nested(nested), Some(counts)));
        }
        _ => return Err(reader.corrupt()),
    };
    reader.end()?;
    Ok((edit, None))
}

fn read_fill(reader: &mut Reader<'_>, after: u64) -> Result<FillParts, PortError> {
    let color = [reader.f32()?, reader.f32()?, reader.f32()?];
    let alpha_locked = reader.bool()?;
    let MaskParts { origin, size, runs } = read_mask(reader)?;
    Ok(FillParts {
        after,
        color,
        alpha_locked,
        origin,
        size,
        runs,
    })
}

fn read_transform(reader: &mut Reader<'_>, after: u64) -> Result<TransformParts, PortError> {
    let lift = match reader.u8()? {
        0 => None,
        1 => Some(read_mask(reader)?),
        _ => return Err(reader.corrupt()),
    };
    let mut values = [0.0f32; 6];
    for value in &mut values {
        *value = reader.f32()?;
    }
    let [a, b, c, d, x, y] = values;
    let patch = match reader.u8()? {
        0 => None,
        1 => {
            let origin = [reader.u32()?, reader.u32()?];
            let size = [reader.u32()?, reader.u32()?];
            let pixels = decode_pixels(reader.rest(), size).ok_or(reader.corrupt())?;
            Some(PatchParts {
                origin,
                size,
                pixels,
            })
        }
        _ => return Err(reader.corrupt()),
    };
    Ok(TransformParts {
        after,
        lift,
        patch,
        matrix: [a, b, c, d],
        translation: [x, y],
    })
}

/// RGBA bytes as a PNG file, quickly compressed: a save writes it once.
fn encode_pixels([width, height]: [u32; 2], rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let encoder = PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Sub);
    if let Err(error) = encoder.write_image(rgba, width, height, ExtendedColorType::Rgba8) {
        // A patch always holds its rectangle's pixels; an empty file reads back as corrupt.
        log::error!("a transform's pixels could not be written: {error}");
        out.clear();
    }
    out
}

/// The RGBA bytes of a PNG file of exactly `size`.
fn decode_pixels(bytes: &[u8], [width, height]: [u32; 2]) -> Option<Vec<u8>> {
    let decoder = PngDecoder::new(Cursor::new(bytes)).ok()?;
    if decoder.dimensions() != (width, height) || decoder.color_type() != image::ColorType::Rgba8 {
        return None;
    }
    let mut pixels = vec![0; usize::try_from(decoder.total_bytes()).ok()?];
    decoder.read_image(&mut pixels).ok()?;
    Some(pixels)
}

/// A mask as fills, clears, transforms, adjustments and clipped strokes store it: its rectangle
/// (x, y, width, height: u32), the run count (u32) and the runs as LEB128 varints.
pub(crate) fn write_mask(writer: &mut Writer, mask: &MaskParts) {
    for value in mask.origin.into_iter().chain(mask.size) {
        writer.u32(value);
    }
    writer.len(mask.runs.len());
    for &run in &mask.runs {
        writer.varint(run);
    }
}

pub(crate) fn read_mask(reader: &mut Reader<'_>) -> Result<MaskParts, PortError> {
    let origin = [reader.u32()?, reader.u32()?];
    let size = [reader.u32()?, reader.u32()?];
    let count = reader.count(1)?;
    let runs = (0..count)
        .map(|_| reader.varint())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MaskParts { origin, size, runs })
}

#[cfg(test)]
#[path = "fills_tests.rs"]
mod tests;
