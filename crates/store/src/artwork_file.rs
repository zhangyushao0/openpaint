//! The OpenPaint file (`.openpaint`, format 2, DECISIONS D62, D125): one artwork as the library
//! stores it, to keep it outside the app and open it again as a new artwork; a kept version
//! (`versions.rs`) is the same bytes. The records are the library's own, so a file is checked by
//! the same decoding as an artwork it opens. The toolbox, history and versions are not in it. A
//! file of format 1 opens converted (`convert/`, this build only).
//!
//! Layout, integers little-endian: the magic `OpenPaint\0` (10 bytes); the format of its records
//! (u32, 2); the title (u32 length + UTF-8); the record count (u32); per record, in key order, its
//! key without the project id (u32 length + bytes: `h` the head, `s` or `e`, slot and index a
//! stroke or an edit, `t` the thumbnail) and its value (u32 length + bytes); last the CRC-32
//! (IEEE) of every byte before it (u32). Its head holds no save id.

use std::collections::BTreeMap;

use openpaint_app::{PortError, ProjectId, Title};
use openpaint_core::Artwork;

use crate::catalogue::decode_thumbnail;
use crate::codec::{Reader, Writer};
use crate::edits::edits_prefix;
use crate::library::{Open, device};
use crate::read::{self, Source};
use crate::records::{FORMAT, head_key, project_prefix, strokes_prefix, thumbnail_key, unsaved};

pub(crate) const MAGIC: &[u8; 10] = b"OpenPaint\0";
/// The kinds of record a file holds, by the first byte of their key.
const KINDS: [u8; 4] = *b"hset";
/// Where an opened file's records are kept while they are decoded.
const HELD: ProjectId = ProjectId::new(1);

/// The records of one file: keys without the project id, and values.
type Records = Vec<(Vec<u8>, Vec<u8>)>;

/// An OpenPaint file, opened and checked.
pub(crate) struct Opened {
    pub(crate) artwork: Artwork,
    pub(crate) title: Title,
    /// The gallery picture's record, when the file has a readable one.
    pub(crate) thumbnail: Option<Vec<u8>>,
}

/// Project `id` as an OpenPaint file: its title and every record but its versions, the head
/// without its save id.
pub(crate) fn of_project(open: &Open, id: ProjectId) -> Result<Vec<u8>, PortError> {
    let title = read::entry(open, id)?.title;
    let prefix = project_prefix(id).len();
    let head = open.projects.get(head_key(id)).map_err(device)?;
    let head = head.ok_or(PortError::Missing(id))?;
    let mut records = vec![(head_key(id), unsaved(head.to_vec()))];
    for kind in [strokes_prefix(id), edits_prefix(id)] {
        open.projects.each(&kind, &mut |key, value| {
            records.push((key.to_vec(), value.to_vec()));
            Ok(())
        })?;
    }
    if let Some(thumbnail) = open.projects.get(thumbnail_key(id)).map_err(device)? {
        records.push((thumbnail_key(id), thumbnail.to_vec()));
    }
    records.sort();
    let relative: Records = records
        .into_iter()
        .map(|(key, value)| (key[prefix..].to_vec(), value))
        .collect();
    Ok(encode(&title, &relative))
}

/// The file of `records` (keys without the project id), titled `title`.
fn encode(title: &Title, records: &Records) -> Vec<u8> {
    let size: usize = records
        .iter()
        .map(|(key, value)| 8 + key.len() + value.len())
        .sum();
    let mut writer = Writer::with_capacity(MAGIC.len() + 16 + title.as_str().len() + size);
    writer.bytes(MAGIC);
    writer.u32(FORMAT);
    writer.str(title.as_str());
    writer.len(records.len());
    for (key, value) in records {
        writer.blob(key);
        writer.blob(value);
    }
    let mut bytes = writer.finish();
    let check = crc32fast::hash(&bytes);
    bytes.extend_from_slice(&check.to_le_bytes());
    bytes
}

/// Opens an OpenPaint file (one of format 1 converted first) and checks the artwork whole.
///
/// # Errors
/// `NotArtworkFile` without the magic, `OtherFormat` for any format but this build's and 1,
/// `Corrupt` or `Invalid` for a damaged file.
pub(crate) fn open(bytes: &[u8]) -> Result<Opened, PortError> {
    let (title, records) = decode(&crate::convert::upgraded(bytes)?)?;
    let prefix = project_prefix(HELD);
    let mut held = BTreeMap::new();
    for (key, value) in records {
        let full = [&prefix[..], &key].concat();
        if held.insert(full, value).is_some() {
            return Err(damaged());
        }
    }
    let artwork = read::artwork(&held, HELD).map_err(|error| match error {
        PortError::Missing(_) => damaged(),
        other => other,
    })?;
    let thumbnail = held
        .remove(&thumbnail_key(HELD))
        .filter(|bytes| decode_thumbnail(bytes).is_ok());
    Ok(Opened {
        artwork,
        title,
        thumbnail,
    })
}

fn damaged() -> PortError {
    PortError::Corrupt {
        what: "OpenPaint file",
    }
}

/// The title and records of a file of this build's format whose magic and check sum hold.
fn decode(bytes: &[u8]) -> Result<(Title, Records), PortError> {
    let body = bytes
        .strip_prefix(&MAGIC[..])
        .ok_or(PortError::NotArtworkFile)?;
    let format = Reader::new(body, "OpenPaint file").u32()?;
    if format != FORMAT {
        return Err(PortError::OtherFormat {
            found: Some(format),
        });
    }
    let (content, check) = bytes.split_last_chunk::<4>().ok_or(damaged())?;
    if content.len() < MAGIC.len() + 4 || crc32fast::hash(content) != u32::from_le_bytes(*check) {
        return Err(damaged());
    }
    let mut reader = Reader::new(&content[MAGIC.len() + 4..], "OpenPaint file");
    let title = Title::new(&reader.str()?).map_err(|_| damaged())?;
    let count = reader.count(8)?;
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let key = reader.blob()?.to_vec();
        if !key.first().is_some_and(|kind| KINDS.contains(kind)) {
            return Err(damaged());
        }
        records.push((key, reader.blob()?.to_vec()));
    }
    reader.end()?;
    Ok((title, records))
}
