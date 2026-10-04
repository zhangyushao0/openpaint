//! Region edit records (schema 8, DECISIONS D36, D45): one per fill, transform or (schema 12)
//! adjustment, keyed like strokes, `[project u64]['f'][layer u64][index u64]`, oldest at index
//! 0. A value starts with its kind (u8: 0 fill, 1 transform, 2 adjustment, `adjusts.rs`).
//!
//! A fill: after (u32: the strokes under it), red, green, blue (f32), alpha locked (u8), its
//! mask (`write_mask`: rectangle, run count, runs as LEB128 varints). A closed shape costs a
//! few bytes per row.
//!
//! A transform: after (u32), what it lifted (u8: 0 the whole layer, 1 a mask as above), its
//! placement (a, b, c, d, then the translation: f32), its patch's rectangle (x, y, width,
//! height: u32), then the patch's premultiplied RGBA pixels as a PNG file to the record's end
//! (lossless; transparent rows cost little).

use std::io::Cursor;

use image::codecs::png::{CompressionType, FilterType, PngDecoder, PngEncoder};
use image::{ExtendedColorType, ImageDecoder, ImageEncoder};
use openpaint_app::{PortError, ProjectId};
use openpaint_core::{EditParts, FillParts, MaskParts, RegionEdit, TransformEdit, TransformParts};

use crate::adjusts::{KIND_ADJUST, encode_adjust, read_adjust};
use crate::codec::{Reader, Writer};
use crate::records::project_prefix;

const FILL: u8 = b'f';
const KIND_FILL: u8 = 0;
const KIND_TRANSFORM: u8 = 1;

/// The prefix of every edit record of `project`.
pub(crate) fn fills_prefix(project: ProjectId) -> Vec<u8> {
    let mut key = project_prefix(project);
    key.push(FILL);
    key
}

/// The prefix of the edit records of one layer, in order.
pub(crate) fn fill_layer_prefix(project: ProjectId, layer: u64) -> Vec<u8> {
    let mut key = fills_prefix(project);
    key.extend_from_slice(&layer.to_be_bytes());
    key
}

pub(crate) fn fill_key(project: ProjectId, layer: u64, index: usize) -> Vec<u8> {
    let mut key = fill_layer_prefix(project, layer);
    key.extend_from_slice(&(index as u64).to_be_bytes());
    key
}

pub(crate) fn encode_edit(edit: &RegionEdit) -> Vec<u8> {
    match edit {
        RegionEdit::Fill(fill) => {
            let mask = fill.mask().to_parts();
            let mut writer = Writer::with_capacity(48 + mask.runs.len() * 2);
            writer.u8(KIND_FILL);
            writer.len(fill.after());
            let color = fill.color();
            for channel in [color.r(), color.g(), color.b()] {
                writer.f32(channel);
            }
            writer.bool(fill.alpha_locked());
            write_mask(&mut writer, &mask);
            writer.finish()
        }
        RegionEdit::Transform(transform) => encode_transform(transform),
        RegionEdit::Adjust(adjust) => encode_adjust(adjust),
    }
}

fn encode_transform(transform: &TransformEdit) -> Vec<u8> {
    let mut writer = Writer::with_capacity(96);
    writer.u8(KIND_TRANSFORM);
    writer.len(transform.after());
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
    let patch = transform.patch();
    for value in patch.origin().into_iter().chain(patch.size()) {
        writer.u32(value);
    }
    writer.bytes(&encode_pixels(patch.size(), patch.pixels()));
    writer.finish()
}

pub(crate) fn decode_edit(bytes: &[u8]) -> Result<EditParts, PortError> {
    let mut reader = Reader::new(bytes, "edit");
    let edit = match reader.u8()? {
        KIND_FILL => EditParts::Fill(read_fill(&mut reader)?),
        KIND_TRANSFORM => EditParts::Transform(read_transform(&mut reader)?),
        KIND_ADJUST => EditParts::Adjust(read_adjust(&mut reader)?),
        _ => return Err(reader.corrupt()),
    };
    reader.end()?;
    Ok(edit)
}

/// A fill after its kind, as schema 5 to 7 stored the whole record.
pub(crate) fn read_fill(reader: &mut Reader<'_>) -> Result<FillParts, PortError> {
    let after = u64::from(reader.u32()?);
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

fn read_transform(reader: &mut Reader<'_>) -> Result<TransformParts, PortError> {
    let after = u64::from(reader.u32()?);
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
    let origin = [reader.u32()?, reader.u32()?];
    let size = [reader.u32()?, reader.u32()?];
    let pixels = decode_pixels(reader.rest(), size).ok_or(reader.corrupt())?;
    Ok(TransformParts {
        after,
        lift,
        origin,
        size,
        pixels,
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

/// A mask as fills and clipped strokes store it: its rectangle (x, y, width, height: u32), the
/// run count (u32) and the runs as LEB128 varints.
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
mod tests {
    use super::*;
    use openpaint_core::{Artwork, CanvasSize, Color, Patch, Placement, RegionMask};

    #[test]
    fn a_fill_record_round_trips_and_damage_is_corrupt() {
        let canvas = CanvasSize::new(300, 4).expect("size");
        let mut inside = vec![false; 1200];
        inside[310..590].fill(true);
        let mask = RegionMask::from_pixels(canvas, &inside).expect("mask");
        let color = Color::rgb(0.25, 0.5, 1.0).expect("colour");
        let artwork = Artwork::new(canvas)
            .fill_region(mask, color)
            .expect("fill")
            .artwork;
        let bytes = encode_edit(&artwork.layers()[0].edits()[0]);
        // One row from x 10 to 289: the runs 0 (one byte) and 280 (two bytes).
        assert_eq!(bytes.len(), 1 + 4 + 12 + 1 + 16 + 4 + 1 + 2);
        let parts = decode_edit(&bytes).expect("decodes");
        assert_eq!(parts, artwork.to_parts().layers[0].edits[0]);
        assert!(decode_edit(&bytes[..bytes.len() - 1]).is_err());
        assert!(decode_edit(&[bytes.clone(), vec![0]].concat()).is_err());
        assert!(
            decode_edit(&[&[7], &bytes[1..]].concat()).is_err(),
            "no such kind"
        );
    }

    #[test]
    fn a_transform_record_keeps_every_pixel_and_damage_is_corrupt() {
        let canvas = CanvasSize::new(40, 30).expect("size");
        let pixels: Vec<u8> = (0..12 * 9 * 4).map(|i| (i * 7 % 256) as u8).collect();
        let patch = Patch::new(canvas, [3, 4], [12, 9], pixels).expect("patch");
        let placement = Placement::around([9.0, 8.0], 1.25, 0.4, [false, true]).expect("map");
        for lift in [
            None,
            RegionMask::polygon(canvas, &[[3.0, 4.0], [15.0, 4.0], [9.0, 13.0]]),
        ] {
            let artwork = Artwork::new(canvas)
                .transform_region(lift, patch.clone(), placement)
                .expect("transform")
                .artwork;
            let bytes = encode_edit(&artwork.layers()[0].edits()[0]);
            assert_eq!(bytes[0], KIND_TRANSFORM);
            let parts = decode_edit(&bytes).expect("decodes");
            assert_eq!(parts, artwork.to_parts().layers[0].edits[0]);
            assert!(
                decode_edit(&bytes[..bytes.len() - 20]).is_err(),
                "a cut file"
            );
        }
    }
}
