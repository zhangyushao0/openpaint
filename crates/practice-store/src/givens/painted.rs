//! A picture the practice crate draws for an exercise (`Painted`) as bytes (S120a), lossless: its
//! width and height (u32 each, at most `MAX_PICTURE_SIDE`, as a pack's), then its pixels row by
//! row as runs of one RGBA colour, each its length (a varint, from 1) and its four bytes, until
//! the picture is full. A flat picture is a few runs a row, never its raw pixels.

use openpaint_app::PortError;
use openpaint_practice::{MAX_PICTURE_SIDE, Painted};
use openpaint_store::{Reader, Writer};

/// `painted`, whose pixels are its width times its height RGBA pixels.
pub(crate) fn write_painted(writer: &mut Writer, (size, pixels): &Painted) {
    writer.u32(size[0]);
    writer.u32(size[1]);
    let mut pixels = pixels.as_chunks::<4>().0.iter();
    let Some(mut colour) = pixels.next() else {
        return;
    };
    let mut run = 1_u32;
    for pixel in pixels {
        if pixel == colour && run < u32::MAX {
            run += 1;
            continue;
        }
        writer.varint(run);
        writer.bytes(colour);
        (colour, run) = (pixel, 1);
    }
    writer.varint(run);
    writer.bytes(colour);
}

/// A picture `write_painted` wrote; a side past `MAX_PICTURE_SIDE`, a run of no pixel or one past
/// the picture's end is corrupt.
pub(crate) fn read_painted(reader: &mut Reader<'_>) -> Result<Painted, PortError> {
    let size = [reader.u32()?, reader.u32()?];
    if size.iter().any(|side| *side > MAX_PICTURE_SIDE) {
        return Err(reader.corrupt());
    }
    let [width, height] = size.map(|side| side as usize);
    let total = width * height;
    // Grown run by run: a damaged size never allocates before its runs are there.
    let mut pixels = Vec::new();
    let mut filled = 0;
    while filled < total {
        let run = reader.varint()? as usize;
        if run == 0 || run > total - filled {
            return Err(reader.corrupt());
        }
        let colour = [reader.u8()?, reader.u8()?, reader.u8()?, reader.u8()?];
        pixels.extend(std::iter::repeat_n(colour, run).flatten());
        filled += run;
    }
    Ok((size, pixels))
}
