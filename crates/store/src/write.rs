//! Writing the library: each function builds one batch and commits it atomically, synced to
//! disk, so a call either happens whole or not at all.

use std::collections::HashSet;

use fjall::{OwnedWriteBatch, PersistMode};
use openpaint_app::{EditorState, PortError, ProjectId, Thumbnail, Title};
use openpaint_core::Artwork;

use crate::catalogue::{Entry, encode_entry, encode_thumbnail};
use crate::delta::delta;
use crate::editor::{EDITOR, encode_editor};
use crate::fills::{encode_fill, fill_key, fills_prefix};
use crate::library::{Open, device};
use crate::read;
use crate::records::{
    CLOCK, LAST, NEXT, decode_id, encode_head, encode_id, encode_stroke, entry_key, head_key,
    project_prefix, stroke_key, strokes_prefix, thumbnail_key,
};

fn commit(batch: OwnedWriteBatch) -> Result<(), PortError> {
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(device)
}

/// The next unused project id, and the batch entry that uses it up.
fn next_id(open: &Open, batch: &mut OwnedWriteBatch) -> Result<ProjectId, PortError> {
    let next = match open.library.get(NEXT).map_err(device)? {
        Some(bytes) => decode_id(&bytes, "next project id")?,
        None => 1,
    };
    batch.insert(&open.library, NEXT, encode_id(next + 1));
    Ok(ProjectId::new(next))
}

/// Writes `id`'s catalogue entry with `title`, touched now: the change counter moves on.
fn touch(
    open: &Open,
    batch: &mut OwnedWriteBatch,
    id: ProjectId,
    title: Title,
) -> Result<(), PortError> {
    let touched = read::clock(open)? + 1;
    batch.insert(&open.library, CLOCK, encode_id(touched));
    let entry = Entry { title, touched };
    batch.insert(&open.library, entry_key(id), encode_entry(&entry));
    Ok(())
}

pub(crate) fn create(
    open: &Open,
    artwork: &Artwork,
    title: &Title,
) -> Result<ProjectId, PortError> {
    let mut batch = open.db.batch();
    let id = next_id(open, &mut batch)?;
    write_all(open, &mut batch, id, artwork)?;
    batch.insert(&open.projects, head_key(id), encode_head(artwork));
    touch(open, &mut batch, id, title.clone())?;
    batch.insert(&open.library, LAST, encode_id(id.get()));
    commit(batch)?;
    Ok(id)
}

/// Project `id` becomes the open one.
pub(crate) fn open(open: &Open, id: ProjectId) -> Result<(), PortError> {
    let mut batch = open.db.batch();
    batch.insert(&open.library, LAST, encode_id(id.get()));
    commit(batch)
}

/// The head and the stroke and fill records that turn `base` (what the disk holds, when known)
/// into `artwork`; without a base, every record. The project is touched and stays open.
pub(crate) fn save(
    open: &Open,
    id: ProjectId,
    base: Option<&Artwork>,
    artwork: &Artwork,
) -> Result<(), PortError> {
    let title = read::entry(open, id)?.title;
    let mut batch = open.db.batch();
    match base {
        Some(base) => write_delta(open, &mut batch, id, base, artwork),
        None => write_all(open, &mut batch, id, artwork)?,
    }
    batch.insert(&open.projects, head_key(id), encode_head(artwork));
    touch(open, &mut batch, id, title)?;
    batch.insert(&open.library, LAST, encode_id(id.get()));
    commit(batch)
}

pub(crate) fn thumbnail(
    open: &Open,
    id: ProjectId,
    thumbnail: &Thumbnail,
) -> Result<(), PortError> {
    read::entry(open, id)?;
    let mut batch = open.db.batch();
    batch.insert(
        &open.projects,
        thumbnail_key(id),
        encode_thumbnail(thumbnail),
    );
    commit(batch)
}

pub(crate) fn editor(open: &Open, state: &EditorState) -> Result<(), PortError> {
    let mut batch = open.db.batch();
    batch.insert(&open.library, EDITOR, encode_editor(state));
    commit(batch)
}

pub(crate) fn close(open: &Open) -> Result<(), PortError> {
    let mut batch = open.db.batch();
    batch.remove(&open.library, LAST);
    commit(batch)
}

/// A new title; the project keeps its place in the gallery.
pub(crate) fn rename(open: &Open, id: ProjectId, title: &Title) -> Result<(), PortError> {
    let entry = Entry {
        title: title.clone(),
        ..read::entry(open, id)?
    };
    let mut batch = open.db.batch();
    batch.insert(&open.library, entry_key(id), encode_entry(&entry));
    commit(batch)
}

/// Every record of `id` copied under a new id, with the same title, touched now.
pub(crate) fn duplicate(open: &Open, id: ProjectId) -> Result<ProjectId, PortError> {
    let title = read::entry(open, id)?.title;
    let mut batch = open.db.batch();
    let copy = next_id(open, &mut batch)?;
    let prefix = project_prefix(id);
    for guard in open.projects.prefix(&prefix) {
        let (key, value) = guard.into_inner().map_err(device)?;
        let mut copied = project_prefix(copy);
        copied.extend_from_slice(&key[prefix.len()..]);
        batch.insert(&open.projects, copied, value);
    }
    touch(open, &mut batch, copy, title)?;
    commit(batch)?;
    Ok(copy)
}

pub(crate) fn delete(open: &Open, id: ProjectId) -> Result<(), PortError> {
    let mut batch = open.db.batch();
    for guard in open.projects.prefix(project_prefix(id)) {
        batch.remove(&open.projects, guard.key().map_err(device)?);
    }
    batch.remove(&open.library, entry_key(id));
    let last = open.library.get(LAST).map_err(device)?;
    if last.is_some_and(|last| decode_id(&last, "last project").ok() == Some(id.get())) {
        batch.remove(&open.library, LAST);
    }
    commit(batch)
}

fn write_delta(
    open: &Open,
    batch: &mut OwnedWriteBatch,
    id: ProjectId,
    base: &Artwork,
    artwork: &Artwork,
) {
    let changes = delta(base, artwork);
    for ((layer, index), stroke) in changes.write {
        batch.insert(
            &open.projects,
            stroke_key(id, layer, index),
            encode_stroke(stroke),
        );
    }
    for (layer, index) in changes.remove {
        batch.remove(&open.projects, stroke_key(id, layer, index));
    }
    for ((layer, index), fill) in changes.write_fills {
        batch.insert(
            &open.projects,
            fill_key(id, layer, index),
            encode_fill(fill),
        );
    }
    for (layer, index) in changes.remove_fills {
        batch.remove(&open.projects, fill_key(id, layer, index));
    }
}

/// Every stroke and fill record of `artwork`, and the removal of any other stroke or fill
/// record the project has on disk. For a new project, and for a save whose base is unknown
/// after a failure.
fn write_all(
    open: &Open,
    batch: &mut OwnedWriteBatch,
    id: ProjectId,
    artwork: &Artwork,
) -> Result<(), PortError> {
    let mut written = HashSet::new();
    for layer in artwork.layers() {
        for (index, stroke) in layer.strokes().iter().enumerate() {
            let key = stroke_key(id, layer.id().get(), index);
            batch.insert(&open.projects, key.clone(), encode_stroke(stroke));
            written.insert(key);
        }
        for (index, fill) in layer.edits().iter().enumerate() {
            let key = fill_key(id, layer.id().get(), index);
            batch.insert(&open.projects, key.clone(), encode_fill(fill));
            written.insert(key);
        }
    }
    for prefix in [strokes_prefix(id), fills_prefix(id)] {
        for guard in open.projects.prefix(prefix) {
            let key = guard.key().map_err(device)?;
            if !written.contains(key.as_ref()) {
                batch.remove(&open.projects, key);
            }
        }
    }
    Ok(())
}
