//! What an exercise gives beside its frame as bytes (S153, `marks.rs`'s codes 19 and 20): a gamut
//! mask, its centre (a point), its radius (f64) and its corners (a list of points); a swatch, its
//! centre, its colour (three u8) and its name's code (u8: 0 none, else one past its key's place
//! in `BESIDE_NAMES`, a code past them refused, never guessed).

use openpaint_app::PortError;
use openpaint_practice::{BESIDE_NAMES, Figure};
use openpaint_store::{Reader, Writer};

use super::marks::{read_colour, read_point, read_points, write_point, write_points};

/// The fields of a gamut mask or a swatch beside the frame; nothing for another figure.
pub(super) fn write_beside(writer: &mut Writer, figure: &Figure) {
    match figure {
        Figure::Gamut {
            at,
            radius,
            corners,
        } => {
            write_point(writer, *at);
            writer.f64(*radius);
            write_points(writer, corners);
        }
        Figure::Beside { at, colour, name } => {
            write_point(writer, *at);
            writer.bytes(colour);
            writer.u8(name_code(*name));
        }
        _ => {}
    }
}

/// A gamut mask beside the frame, its code read.
pub(super) fn read_gamut(reader: &mut Reader<'_>) -> Result<Figure, PortError> {
    let (at, radius) = (read_point(reader)?, reader.f64()?);
    let corners = read_points(reader)?;
    Ok(Figure::Gamut {
        at,
        radius,
        corners,
    })
}

/// A swatch beside the frame, its code read.
pub(super) fn read_beside(reader: &mut Reader<'_>) -> Result<Figure, PortError> {
    let (at, colour) = (read_point(reader)?, read_colour(reader)?);
    let name = read_name(reader)?;
    Ok(Figure::Beside { at, colour, name })
}

/// A swatch's name as stored.
fn name_code(name: Option<&str>) -> u8 {
    let place = name.and_then(|name| BESIDE_NAMES.iter().position(|key| *key == name));
    place.map_or(0, |at| u8::try_from(at + 1).unwrap_or(0))
}

/// The swatch's name stored as its code.
fn read_name(reader: &mut Reader<'_>) -> Result<Option<&'static str>, PortError> {
    match usize::from(reader.u8()?) {
        0 => Ok(None),
        code => BESIDE_NAMES
            .get(code - 1)
            .map(|name| Some(*name))
            .ok_or(reader.corrupt()),
    }
}
