//! The OpenPaint file (`.openpaint`, DECISIONS D62): one artwork as the library stores it, to
//! keep it outside the app and open it again as a new artwork; a kept version (`versions.rs`)
//! is the same bytes. The records are the library's own, so a file migrates through the same
//! schema steps as a library and is checked by the same decoding as an artwork it opens. The
//! toolbox, history and versions are not in it.
//!
//! Layout, integers little-endian: the magic `OpenPaint\0` (10 bytes); the schema of its
//! records (u32); the title (u32 length + UTF-8); the record count (u32); per record, in key
//! order, its key without the project id (u32 length + bytes: `h` the head, `s` or `f`, layer
//! and index a stroke or an edit, `t` the thumbnail) and its value (u32 length + bytes); last the
//! CRC-32 (IEEE) of every byte before it (u32).

use std::collections::BTreeMap;

use openpaint_app::{PortError, ProjectId, Title};
use openpaint_core::Artwork;

use crate::catalogue::decode_thumbnail;
use crate::codec::{Reader, Writer};
use crate::fills::fills_prefix;
use crate::library::{Open, device};
use crate::read::{self, Source};
use crate::records::{
    SCHEMA, head_key, head_schema, project_prefix, strokes_prefix, thumbnail_key,
};
use crate::steps::{Old, Records, run_guarded};

const MAGIC: &[u8; 10] = b"OpenPaint\0";
/// The kinds of record a file holds, by the first byte of their key.
const KINDS: [u8; 4] = *b"hsft";
/// Where an opened file's records are kept while they are upgraded and decoded.
const HELD: ProjectId = ProjectId::new(1);

/// An OpenPaint file, opened and checked.
pub(crate) struct Opened {
    pub(crate) artwork: Artwork,
    pub(crate) title: Title,
    /// The gallery picture's record, when the file has a readable one.
    pub(crate) thumbnail: Option<Vec<u8>>,
}

/// Project `id` as an OpenPaint file: its title and every record but its versions, of the schema
/// its head is marked with (an artwork whose upgrade failed keeps its own, S50, so a build whose
/// steps read it can open the file).
pub(crate) fn of_project(open: &Open, id: ProjectId) -> Result<Vec<u8>, PortError> {
    let title = read::entry(open, id)?.title;
    let prefix = project_prefix(id).len();
    let head = open.projects.get(head_key(id)).map_err(device)?;
    let head = head.ok_or(PortError::Missing(id))?;
    let schema = head_schema(&head).filter(|schema| (1..=SCHEMA).contains(schema));
    let mut records = vec![(head_key(id), head.to_vec())];
    for kind in [strokes_prefix(id), fills_prefix(id)] {
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
    Ok(encode(schema.unwrap_or(SCHEMA), &title, &relative))
}

/// The file of `records` (keys without the project id) of `schema`, titled `title`.
pub(crate) fn encode(schema: u32, title: &Title, records: &Records) -> Vec<u8> {
    let size: usize = records
        .iter()
        .map(|(key, value)| 8 + key.len() + value.len())
        .sum();
    let mut writer = Writer::with_capacity(MAGIC.len() + 16 + title.as_str().len() + size);
    writer.bytes(MAGIC);
    writer.u32(schema);
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

/// Opens an OpenPaint file: records of an older schema are upgraded, the artwork is checked
/// whole.
///
/// # Errors
/// `NotArtworkFile` without the magic, `NewerSchema` for a newer schema, `Corrupt` or `Invalid`
/// for a damaged file.
pub(crate) fn open(bytes: &[u8]) -> Result<Opened, PortError> {
    let (schema, title, records) = decode(bytes)?;
    let prefix = project_prefix(HELD);
    let mut held = BTreeMap::new();
    for (key, value) in records {
        let full = [&prefix[..], &key].concat();
        if held.insert(full, value).is_some() {
            return Err(damaged());
        }
    }
    if schema < SCHEMA {
        upgrade(schema, &mut held)?;
    }
    let artwork = read::artwork(&held, HELD).map_err(|error| match error {
        PortError::Missing(_) | PortError::NotUpgraded { .. } => damaged(),
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

/// The schema, title and records of a file whose magic and check sum hold.
fn decode(bytes: &[u8]) -> Result<(u32, Title, Records), PortError> {
    let body = bytes
        .strip_prefix(&MAGIC[..])
        .ok_or(PortError::NotArtworkFile)?;
    let mut reader = Reader::new(body, "OpenPaint file");
    let schema = reader.u32()?;
    if schema > SCHEMA {
        return Err(PortError::NewerSchema {
            found: schema,
            newest: SCHEMA,
        });
    }
    let (content, check) = bytes.split_last_chunk::<4>().ok_or(damaged())?;
    if schema == 0 || crc32fast::hash(content) != u32::from_le_bytes(*check) {
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
    Ok((schema, title, records))
}

/// Runs the schema steps from `schema` over the held records, as a library's upgrade does; a step
/// that fails on them makes the file damaged.
fn upgrade(schema: u32, held: &mut BTreeMap<Vec<u8>, Vec<u8>>) -> Result<(), PortError> {
    let head = held.get(&head_key(HELD)).ok_or(damaged())?.clone();
    let under = |prefix: Vec<u8>| -> Records {
        held.iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    };
    let old = Old {
        heads: vec![(HELD, head)],
        strokes: under(strokes_prefix(HELD)),
        fills: under(fills_prefix(HELD)),
        editor: None,
    };
    held.extend(run_guarded(schema, old).ok_or_else(damaged)?.projects);
    Ok(())
}
