//! Schema 2's gallery records (`records.rs` has the keys): a catalogue entry per project in
//! `library` and a thumbnail per project in `projects`.
//!
//! - Entry: title (u32 length + UTF-8), touched (u64): the value of the library's change
//!   counter when the project was created, duplicated or saved last; the gallery lists the
//!   highest first.
//! - Thumbnail: width and height (u32), then width x height RGBA pixels, rows top first.

use openpaint_app::{PortError, Thumbnail, Title};

use crate::codec::{Reader, Writer};

/// A project as the catalogue holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) title: Title,
    pub(crate) touched: u64,
}

pub(crate) fn encode_entry(entry: &Entry) -> Vec<u8> {
    let title = entry.title.as_str();
    let mut writer = Writer::with_capacity(12 + title.len());
    writer.str(title);
    writer.u64(entry.touched);
    writer.finish()
}

pub(crate) fn decode_entry(bytes: &[u8]) -> Result<Entry, PortError> {
    let mut reader = Reader::new(bytes, "catalogue entry");
    let title = Title::new(&reader.str()?).map_err(|_| reader.corrupt())?;
    let touched = reader.u64()?;
    reader.end()?;
    Ok(Entry { title, touched })
}

pub(crate) fn encode_thumbnail(thumbnail: &Thumbnail) -> Vec<u8> {
    let pixels = thumbnail.pixels();
    let mut writer = Writer::with_capacity(8 + pixels.len());
    let [width, height] = thumbnail.size();
    writer.u32(width);
    writer.u32(height);
    writer.bytes(pixels);
    writer.finish()
}

pub(crate) fn decode_thumbnail(bytes: &[u8]) -> Result<Thumbnail, PortError> {
    let mut reader = Reader::new(bytes, "thumbnail");
    let size = [reader.u32()?, reader.u32()?];
    let pixels = reader.rest().to_vec();
    Thumbnail::new(size, pixels).ok_or(reader.corrupt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_and_thumbnails_round_trip_and_bad_ones_are_corrupt() {
        let entry = Entry {
            title: Title::new("Hafen ✒ 墨").expect("title"),
            touched: 7,
        };
        assert_eq!(decode_entry(&encode_entry(&entry)), Ok(entry));
        let mut writer = Writer::default();
        writer.str("");
        writer.u64(1);
        assert!(decode_entry(&writer.finish()).is_err(), "an empty title");
        let thumbnail = Thumbnail::new([2, 1], (0..8).collect()).expect("thumbnail");
        let bytes = encode_thumbnail(&thumbnail);
        assert_eq!(decode_thumbnail(&bytes), Ok(thumbnail));
        assert!(decode_thumbnail(&bytes[..bytes.len() - 1]).is_err());
    }
}
