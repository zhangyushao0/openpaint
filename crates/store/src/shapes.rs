//! The shape stage of a stroke's recipe (schema 20, DECISIONS D72; the layout is in
//! `records.rs`): stamp (u8: 0 disc, 1 square, 2 ellipse, then the shape textures 3 chalk, 4
//! grainy, 5 splatter, 6 rough square), aspect, spacing, scatter (f32), rotation (u8: 0 fixed, 1
//! the stroke's direction, 2 the pen's azimuth), angle, rotation jitter (f32, degrees) and size
//! jitter (f32).

use openpaint_app::PortError;
use openpaint_core::{Rotation, ShapeParts, ShapeTexture, Stamp};

use crate::codec::{Reader, Writer};

pub(crate) fn write_shape(writer: &mut Writer, shape: &ShapeParts) {
    writer.u8(match shape.stamp {
        Stamp::Disc => 0,
        Stamp::Square => 1,
        Stamp::Ellipse => 2,
        Stamp::Texture(ShapeTexture::Chalk) => 3,
        Stamp::Texture(ShapeTexture::Grainy) => 4,
        Stamp::Texture(ShapeTexture::Splatter) => 5,
        Stamp::Texture(ShapeTexture::RoughSquare) => 6,
    });
    for value in [shape.aspect, shape.spacing, shape.scatter] {
        writer.f32(value);
    }
    writer.u8(match shape.rotation {
        Rotation::Fixed => 0,
        Rotation::Direction => 1,
        Rotation::Azimuth => 2,
    });
    for value in [shape.angle, shape.rotation_jitter, shape.size_jitter] {
        writer.f32(value);
    }
}

/// A shape stage as `write_shape` stores it; an unknown stamp or rotation is corrupt.
pub(crate) fn read_shape(reader: &mut Reader<'_>) -> Result<ShapeParts, PortError> {
    let stamp = match reader.u8()? {
        0 => Stamp::Disc,
        1 => Stamp::Square,
        2 => Stamp::Ellipse,
        3 => Stamp::Texture(ShapeTexture::Chalk),
        4 => Stamp::Texture(ShapeTexture::Grainy),
        5 => Stamp::Texture(ShapeTexture::Splatter),
        6 => Stamp::Texture(ShapeTexture::RoughSquare),
        _ => return Err(reader.corrupt()),
    };
    let (aspect, spacing, scatter) = (reader.f32()?, reader.f32()?, reader.f32()?);
    let rotation = match reader.u8()? {
        0 => Rotation::Fixed,
        1 => Rotation::Direction,
        2 => Rotation::Azimuth,
        _ => return Err(reader.corrupt()),
    };
    Ok(ShapeParts {
        stamp,
        aspect,
        spacing,
        scatter,
        rotation,
        angle: reader.f32()?,
        rotation_jitter: reader.f32()?,
        size_jitter: reader.f32()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bytes of a shape stage.
    const SHAPE_BYTES: usize = 1 + 3 * 4 + 1 + 3 * 4;

    #[test]
    fn every_stamp_and_rotation_round_trips_and_unknown_bytes_are_corrupt() {
        let stamps = [Stamp::Disc, Stamp::Square, Stamp::Ellipse]
            .into_iter()
            .chain(ShapeTexture::ALL.map(Stamp::Texture));
        let rotations = [Rotation::Fixed, Rotation::Direction, Rotation::Azimuth];
        for (at, stamp) in stamps.enumerate() {
            let shape = ShapeParts {
                stamp,
                aspect: 0.5,
                spacing: 0.25,
                scatter: 1.5,
                rotation: rotations[at % 3],
                angle: -45.0,
                rotation_jitter: 30.0,
                size_jitter: 0.125,
            };
            let mut writer = Writer::default();
            write_shape(&mut writer, &shape);
            let bytes = writer.finish();
            assert_eq!(bytes.len(), SHAPE_BYTES);
            assert_eq!(usize::from(bytes[0]), at, "the stamp's byte");
            let mut reader = Reader::new(&bytes, "stroke");
            assert_eq!(read_shape(&mut reader), Ok(shape));
            assert_eq!(reader.end(), Ok(()));
            for (place, byte) in [(0, 7), (13, 3)] {
                let mut unknown = bytes.clone();
                unknown[place] = byte;
                let read = read_shape(&mut Reader::new(&unknown, "stroke"));
                assert_eq!(read, Err(PortError::Corrupt { what: "stroke" }));
            }
        }
    }
}
