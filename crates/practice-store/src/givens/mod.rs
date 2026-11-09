//! What each exercise of a sheet printed, kept as bytes (S120a): its givens (`Exercise::givens`)
//! and the picture its kind draws (`Exercise::painted`), so a sheet shows what it printed whatever
//! became of the generator. Lossless: `decode(encode(x)) == x`, every float its own bits.
//!
//! A sheet's givens are one blob, written once when its lessons are laid: the format (u8,
//! `GIVENS_FORMAT`), a slot count (u32) and per slot its number (u16), its marks (`marks.rs`) and
//! its painted picture, a flag (u8, 0 none, 1 one) and the picture (`painted.rs`). The marks and
//! the picture are also encoded alone, without the format byte.

mod marks;
mod painted;

use openpaint_app::PortError;
use openpaint_practice::{Given, Givens, Painted, SheetMark};
use openpaint_store::{Reader, Writer};

/// The layout of a sheet's givens this build reads and writes.
pub const GIVENS_FORMAT: u8 = 1;
/// The fewest bytes of a slot: its number, an empty list of marks and no picture.
const SLOT_BYTES: usize = 2 + 4 + 1;
const WHAT: &str = "practice givens";

#[must_use]
pub fn encode_givens(givens: &Givens) -> Vec<u8> {
    let mut writer = Writer::default();
    writer.u8(GIVENS_FORMAT);
    writer.len(givens.slots.len());
    for (slot, given) in &givens.slots {
        writer.u16(*slot);
        marks::write_marks(&mut writer, &given.marks);
        match &given.painted {
            None => writer.u8(0),
            Some(painted) => {
                writer.u8(1);
                painted::write_painted(&mut writer, painted);
            }
        }
    }
    writer.finish()
}

/// # Errors
/// `OtherFormat` for givens of another format; `Corrupt` for bytes `encode_givens` did not write:
/// cut short, longer, or a code this build does not know.
pub fn decode_givens(bytes: &[u8]) -> Result<Givens, PortError> {
    let mut reader = Reader::new(bytes, WHAT);
    let format = reader.u8()?;
    if format != GIVENS_FORMAT {
        let found = Some(u32::from(format));
        return Err(PortError::OtherFormat { found });
    }
    let count = reader.count(SLOT_BYTES)?;
    let mut slots = Vec::with_capacity(count);
    for _ in 0..count {
        let slot = reader.u16()?;
        let marks = marks::read_marks(&mut reader)?;
        let painted = match reader.u8()? {
            0 => None,
            1 => Some(painted::read_painted(&mut reader)?),
            _ => return Err(reader.corrupt()),
        };
        slots.push((slot, Given { marks, painted }));
    }
    reader.end()?;
    Ok(Givens { slots })
}

#[must_use]
pub fn encode_marks(marks: &[SheetMark]) -> Vec<u8> {
    let mut writer = Writer::default();
    marks::write_marks(&mut writer, marks);
    writer.finish()
}

/// # Errors
/// `Corrupt` for bytes `encode_marks` did not write.
pub fn decode_marks(bytes: &[u8]) -> Result<Vec<SheetMark>, PortError> {
    let mut reader = Reader::new(bytes, WHAT);
    let marks = marks::read_marks(&mut reader)?;
    reader.end()?;
    Ok(marks)
}

/// `painted`'s bytes; its pixels are its width times its height RGBA pixels, as the practice
/// crate draws it (any other count does not decode).
#[must_use]
pub fn encode_painted(painted: &Painted) -> Vec<u8> {
    let mut writer = Writer::default();
    painted::write_painted(&mut writer, painted);
    writer.finish()
}

/// # Errors
/// `Corrupt` for bytes `encode_painted` did not write.
pub fn decode_painted(bytes: &[u8]) -> Result<Painted, PortError> {
    let mut reader = Reader::new(bytes, WHAT);
    let painted = painted::read_painted(&mut reader)?;
    reader.end()?;
    Ok(painted)
}
