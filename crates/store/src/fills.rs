//! Fill records (schema 5, DECISIONS D36): one per fill, keyed like strokes,
//! `[project u64]['f'][layer u64][index u64]`, oldest at index 0. A value is: after (u32: the
//! strokes under it), red, green, blue (f32), alpha locked (u8), the mask's rectangle (x, y,
//! width, height: u32), the run count (u32) and the runs, each a LEB128 varint: alternately
//! pixels outside and inside the mask, row by row over the rectangle, starting outside.
//! A closed shape costs a few bytes per row.

use openpaint_app::{PortError, ProjectId};
use openpaint_core::{FillParts, RegionEdit};

use crate::codec::{Reader, Writer};
use crate::records::project_prefix;

const FILL: u8 = b'f';

/// The prefix of every fill record of `project`.
pub(crate) fn fills_prefix(project: ProjectId) -> Vec<u8> {
    let mut key = project_prefix(project);
    key.push(FILL);
    key
}

/// The prefix of the fill records of one layer, in order.
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

pub(crate) fn encode_fill(edit: &RegionEdit) -> Vec<u8> {
    let mask = edit.mask();
    let runs = mask.runs();
    let mut writer = Writer::with_capacity(48 + runs.len() * 2);
    writer.len(edit.after());
    let color = edit.color();
    for channel in [color.r(), color.g(), color.b()] {
        writer.f32(channel);
    }
    writer.bool(edit.alpha_locked());
    for value in mask.origin().into_iter().chain(mask.size()) {
        writer.u32(value);
    }
    writer.len(runs.len());
    for run in runs {
        writer.varint(run);
    }
    writer.finish()
}

pub(crate) fn decode_fill(bytes: &[u8]) -> Result<FillParts, PortError> {
    let mut reader = Reader::new(bytes, "fill");
    let after = u64::from(reader.u32()?);
    let color = [reader.f32()?, reader.f32()?, reader.f32()?];
    let alpha_locked = reader.bool()?;
    let origin = [reader.u32()?, reader.u32()?];
    let size = [reader.u32()?, reader.u32()?];
    let count = reader.count(1)?;
    let runs = (0..count)
        .map(|_| reader.varint())
        .collect::<Result<Vec<_>, _>>()?;
    reader.end()?;
    Ok(FillParts {
        after,
        color,
        alpha_locked,
        origin,
        size,
        runs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_core::{Artwork, CanvasSize, Color, RegionMask};

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
        let edit = &artwork.layers()[0].edits()[0];
        let bytes = encode_fill(edit);
        // One row from x 10 to 289: the runs 0 (one byte) and 280 (two bytes).
        assert_eq!(bytes.len(), 4 + 12 + 1 + 16 + 4 + 1 + 2);
        let parts = decode_fill(&bytes).expect("decodes");
        assert_eq!(parts, artwork.to_parts().layers[0].fills[0]);
        assert!(decode_fill(&bytes[..bytes.len() - 1]).is_err());
        assert!(decode_fill(&[bytes.clone(), vec![0]].concat()).is_err());
    }
}
