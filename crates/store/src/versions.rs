//! Kept versions (DECISIONS D63): an artwork as it was stored when it was left, kept as the
//! bytes of an OpenPaint file (`artwork_file.rs`), at most `KEPT_VERSIONS` per artwork, the
//! oldest dropped. A version carries its own schema, so library upgrades never touch it and a
//! restore migrates it as a file is; restoring makes a new artwork and changes nothing kept.
//!
//! - `library`: `['v'][project u64][version u64]` (big-endian) -> when it was kept (u64 LE,
//!   seconds since 1970 UTC); small, so the gallery lists versions without reading them.
//! - `projects`: `[project u64]['v'][version u64]` -> the file's bytes; with the project's other
//!   records, so deleting the project deletes them.
//!
//! Version numbers count up per project from 1 and are never reused.

use openpaint_app::{KEPT_VERSIONS, KeptVersion, PortError, ProjectId, VersionId};

use crate::artwork_file;
use crate::codec::Reader;
use crate::library::{Open, commit, device};
use crate::records::project_prefix;

/// The first byte of a version's keys.
const VERSION: u8 = b'v';

/// The prefix of project `id`'s version entries in `library`.
pub(crate) fn entries_prefix(id: ProjectId) -> Vec<u8> {
    let mut key = vec![VERSION];
    key.extend_from_slice(&id.get().to_be_bytes());
    key
}

fn entry_key(id: ProjectId, version: VersionId) -> Vec<u8> {
    let mut key = entries_prefix(id);
    key.extend_from_slice(&version.get().to_be_bytes());
    key
}

fn file_key(id: ProjectId, version: VersionId) -> Vec<u8> {
    let mut key = project_prefix(id);
    key.push(VERSION);
    key.extend_from_slice(&version.get().to_be_bytes());
    key
}

/// Whether `key` (under a project's prefix in `projects`) is one of its versions.
pub(crate) fn is_version(key: &[u8]) -> bool {
    key.get(8) == Some(&VERSION)
}

/// Project `id`'s versions, the newest first.
pub(crate) fn listed(open: &Open, id: ProjectId) -> Result<Vec<KeptVersion>, PortError> {
    let prefix = entries_prefix(id);
    let mut versions = Vec::new();
    for guard in open.library.prefix(&prefix) {
        let (key, value) = guard.into_inner().map_err(device)?;
        let corrupt = PortError::Corrupt {
            what: "version entry",
        };
        let number = key[prefix.len()..]
            .try_into()
            .map_err(|_| corrupt.clone())?;
        let mut reader = Reader::new(&value, "version entry");
        let kept_at = reader.u64()?;
        reader.end()?;
        versions.push(KeptVersion {
            id: VersionId::new(u64::from_be_bytes(number)),
            kept_at,
        });
    }
    versions.reverse();
    Ok(versions)
}

/// Keeps project `id` as stored now as a new version kept at `kept_at`, dropping the oldest
/// beyond `KEPT_VERSIONS`, in one batch.
pub(crate) fn keep(open: &Open, id: ProjectId, kept_at: u64) -> Result<(), PortError> {
    let bytes = artwork_file::of_project(open, id)?;
    let kept = listed(open, id)?;
    let next = kept.first().map_or(1, |newest| newest.id.get() + 1);
    let version = VersionId::new(next);
    let mut batch = open.db.batch();
    batch.insert(
        &open.library,
        entry_key(id, version),
        kept_at.to_le_bytes().to_vec(),
    );
    batch.insert(&open.projects, file_key(id, version), bytes.as_slice());
    for old in kept.iter().skip(KEPT_VERSIONS - 1) {
        batch.remove(&open.library, entry_key(id, old.id));
        batch.remove(&open.projects, file_key(id, old.id));
    }
    commit(batch)?;
    log::info!(
        "project {} kept as version {next} ({} bytes)",
        id.get(),
        bytes.len()
    );
    Ok(())
}

/// The OpenPaint file of version `version` of project `id`.
pub(crate) fn file(open: &Open, id: ProjectId, version: VersionId) -> Result<Vec<u8>, PortError> {
    let bytes = open
        .projects
        .get(file_key(id, version))
        .map_err(device)?
        .ok_or(PortError::Missing(id))?;
    Ok(bytes.to_vec())
}
