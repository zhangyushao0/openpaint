//! Writing the library: each function builds one batch and commits it atomically, synced to
//! disk (`batch.rs`), so a call either happens whole or not at all.

use std::collections::HashSet;
use std::hash::{BuildHasher, RandomState};
use std::sync::atomic::{AtomicU64, Ordering};

use openpaint_app::{EditorState, PortError, ProjectId, SaveId, Thumbnail, Title};
use openpaint_core::Artwork;
use openpaint_practice::SessionRecord;

use crate::artwork_file::Opened;
use crate::batch::Batch;
use crate::catalogue::{Entry, encode_entry, encode_thumbnail};
use crate::delta::{contents, delta};
use crate::editor::{EDITOR, encode_editor};
use crate::edits::{edit_key, edits_prefix, encode_edit};
use crate::library::{Open, device};
use crate::practice::{encode_session, practice_key};
use crate::read;
use crate::records::{
    CLOCK, LAST, NEXT, decode_id, encode_head, encode_id, entry_key, head_key, project_prefix,
    stroke_key, strokes_prefix, thumbnail_key,
};
use crate::strokes::encode_stroke;
use crate::versions::{entries_prefix, is_version};

/// The next unused project id, and the batch entry that uses it up.
fn next_id(open: &Open, batch: &mut Batch) -> Result<ProjectId, PortError> {
    let next = match open.library.get(NEXT).map_err(device)? {
        Some(bytes) => decode_id(&bytes, "next project id")?,
        None => 1,
    };
    batch.insert(&open.library, NEXT, encode_id(next + 1));
    Ok(ProjectId::new(next))
}

/// Writes `id`'s catalogue entry with `title`, touched now: the change counter moves on.
fn touch(open: &Open, batch: &mut Batch, id: ProjectId, title: Title) -> Result<(), PortError> {
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
    let mut batch = Batch::new(open);
    let id = new_project(open, &mut batch, artwork, title)?;
    batch.insert(&open.library, LAST, encode_id(id.get()));
    batch.commit()?;
    Ok(id)
}

/// An opened OpenPaint file as a new project, listed first, titled `title` or as the file
/// says, with the file's thumbnail; the open project stays open.
pub(crate) fn add(
    open: &Open,
    opened: &Opened,
    title: Option<&Title>,
) -> Result<ProjectId, PortError> {
    let mut batch = Batch::new(open);
    let title = title.unwrap_or(&opened.title);
    let id = new_project(open, &mut batch, &opened.artwork, title)?;
    if let Some(thumbnail) = &opened.thumbnail {
        batch.insert(&open.projects, thumbnail_key(id), thumbnail.as_slice());
    }
    batch.commit()?;
    Ok(id)
}

/// Every record of `artwork` under a new id, its entry titled `title`, touched now.
fn new_project(
    open: &Open,
    batch: &mut Batch,
    artwork: &Artwork,
    title: &Title,
) -> Result<ProjectId, PortError> {
    let id = next_id(open, batch)?;
    write_all(open, batch, id, artwork)?;
    // No save id until it is opened or saved.
    batch.insert(&open.projects, head_key(id), encode_head(artwork, None));
    touch(open, batch, id, title.clone())?;
    Ok(id)
}

/// A new practice session: `sheet` as a new project titled `title`, which becomes the open one,
/// and `session`'s record naming it, in one batch.
pub(crate) fn start_practice(
    open: &Open,
    (sheet, title): (&Artwork, &Title),
    session: &SessionRecord,
) -> Result<ProjectId, PortError> {
    let mut batch = Batch::new(open);
    let id = new_project(open, &mut batch, sheet, title)?;
    batch.insert(&open.library, LAST, encode_id(id.get()));
    let session = SessionRecord {
        canvas: id.get(),
        ..session.clone()
    };
    let key = practice_key(session.id);
    batch.insert(&open.library, key, encode_session(&session));
    batch.commit()?;
    Ok(id)
}

/// Replaces `session`'s record.
pub(crate) fn practice(open: &Open, session: &SessionRecord) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    let key = practice_key(session.id);
    batch.insert(&open.library, key, encode_session(session));
    batch.commit()
}

/// Project `id` becomes the open one.
pub(crate) fn open(open: &Open, id: ProjectId) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    batch.insert(&open.library, LAST, encode_id(id.get()));
    batch.commit()
}

/// Project `id`'s head, which holds `artwork` and no save id, written again with a new one
///, which is returned; nothing else changes.
pub(crate) fn name(open: &Open, id: ProjectId, artwork: &Artwork) -> Result<SaveId, PortError> {
    let saved = new_save_id();
    let mut batch = Batch::new(open);
    batch.insert(
        &open.projects,
        head_key(id),
        encode_head(artwork, Some(saved)),
    );
    batch.commit()?;
    Ok(saved)
}

/// The head and the stroke and edit records that turn `base` (what the disk holds, when known)
/// into `artwork`; without a base, every record. The head holds a new save id, returned. The
/// project is touched and stays open.
pub(crate) fn save(
    open: &Open,
    id: ProjectId,
    base: Option<&Artwork>,
    artwork: &Artwork,
) -> Result<SaveId, PortError> {
    let title = read::entry(open, id)?.title;
    let mut batch = Batch::new(open);
    match base {
        Some(base) => write_delta(open, &mut batch, id, base, artwork),
        None => write_all(open, &mut batch, id, artwork)?,
    }
    let saved = new_save_id();
    batch.insert(
        &open.projects,
        head_key(id),
        encode_head(artwork, Some(saved)),
    );
    touch(open, &mut batch, id, title)?;
    batch.insert(&open.library, LAST, encode_id(id.get()));
    batch.commit()?;
    Ok(saved)
}

/// A new random save id: SipHash of a count under keys `RandomState` takes from the
/// system's randomness once a thread, so two saves, in this process or any other, share one
/// only by a chance of about 2^-64.
fn new_save_id() -> SaveId {
    static SAVES: AtomicU64 = AtomicU64::new(0);
    loop {
        let count = SAVES.fetch_add(1, Ordering::Relaxed);
        if let Some(saved) = SaveId::new(RandomState::new().hash_one(count)) {
            return saved;
        }
    }
}

pub(crate) fn thumbnail(
    open: &Open,
    id: ProjectId,
    thumbnail: &Thumbnail,
) -> Result<(), PortError> {
    read::entry(open, id)?;
    let mut batch = Batch::new(open);
    batch.insert(
        &open.projects,
        thumbnail_key(id),
        encode_thumbnail(thumbnail),
    );
    batch.commit()
}

pub(crate) fn editor(open: &Open, state: &EditorState) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    batch.insert(&open.library, EDITOR, encode_editor(state));
    batch.commit()
}

pub(crate) fn close(open: &Open) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    batch.remove(&open.library, LAST);
    batch.commit()
}

/// A new title; the project keeps its place in the gallery.
pub(crate) fn rename(open: &Open, id: ProjectId, title: &Title) -> Result<(), PortError> {
    let entry = Entry {
        title: title.clone(),
        ..read::entry(open, id)?
    };
    let mut batch = Batch::new(open);
    batch.insert(&open.library, entry_key(id), encode_entry(&entry));
    batch.commit()
}

/// Every record of `id` but its versions copied under a new id, with the same title, touched
/// now.
pub(crate) fn duplicate(open: &Open, id: ProjectId) -> Result<ProjectId, PortError> {
    let title = read::entry(open, id)?.title;
    let mut batch = Batch::new(open);
    let copy = next_id(open, &mut batch)?;
    let prefix = project_prefix(id);
    for guard in open.projects.prefix(&prefix) {
        let (key, value) = guard.into_inner().map_err(device)?;
        if is_version(&key) {
            continue;
        }
        let mut copied = project_prefix(copy);
        copied.extend_from_slice(&key[prefix.len()..]);
        batch.insert(&open.projects, copied, value);
    }
    touch(open, &mut batch, copy, title)?;
    batch.commit()?;
    Ok(copy)
}

pub(crate) fn delete(open: &Open, id: ProjectId) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    remove_project(open, &mut batch, id)?;
    batch.commit()
}

/// `session`'s record and its sheet removed, in one batch.
pub(crate) fn remove_practice(open: &Open, session: &SessionRecord) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    batch.remove(&open.library, practice_key(session.id));
    if session.canvas != 0 {
        remove_project(open, &mut batch, ProjectId::new(session.canvas))?;
    }
    batch.commit()
}

/// Every record of project `id` removed: its own, its catalogue entry, its kept versions, and
/// `last` if it names it.
fn remove_project(open: &Open, batch: &mut Batch, id: ProjectId) -> Result<(), PortError> {
    for guard in open.projects.prefix(project_prefix(id)) {
        batch.remove(&open.projects, guard.key().map_err(device)?);
    }
    batch.remove(&open.library, entry_key(id));
    for guard in open.library.prefix(entries_prefix(id)) {
        batch.remove(&open.library, guard.key().map_err(device)?);
    }
    let last = open.library.get(LAST).map_err(device)?;
    if last.is_some_and(|last| decode_id(&last, "last project").ok() == Some(id.get())) {
        batch.remove(&open.library, LAST);
    }
    Ok(())
}

fn write_delta(open: &Open, batch: &mut Batch, id: ProjectId, base: &Artwork, artwork: &Artwork) {
    let changes = delta(base, artwork);
    for ((slot, index), stroke) in changes.strokes {
        batch.insert(
            &open.projects,
            stroke_key(id, slot, index),
            encode_stroke(stroke),
        );
    }
    for (slot, index) in changes.removed_strokes {
        batch.remove(&open.projects, stroke_key(id, slot, index));
    }
    for ((slot, index), edit) in changes.edits {
        batch.insert(&open.projects, edit_key(id, slot, index), encode_edit(edit));
    }
    for (slot, index) in changes.removed_edits {
        batch.remove(&open.projects, edit_key(id, slot, index));
    }
}

/// Every stroke and edit record of `artwork`, and the removal of any other stroke or edit
/// record the project has on disk. For a new project, and for a save whose base is unknown
/// after a failure.
fn write_all(
    open: &Open,
    batch: &mut Batch,
    id: ProjectId,
    artwork: &Artwork,
) -> Result<(), PortError> {
    let mut written = HashSet::new();
    for (slot, strokes, edits) in contents(artwork) {
        for (index, stroke) in strokes.iter().enumerate() {
            let key = stroke_key(id, slot, index);
            batch.insert(&open.projects, key.clone(), encode_stroke(stroke));
            written.insert(key);
        }
        for (index, edit) in edits.iter().enumerate() {
            let key = edit_key(id, slot, index);
            batch.insert(&open.projects, key.clone(), encode_edit(edit));
            written.insert(key);
        }
    }
    for prefix in [strokes_prefix(id), edits_prefix(id)] {
        for guard in open.projects.prefix(prefix) {
            let key = guard.key().map_err(device)?;
            if !written.contains(key.as_ref()) {
                batch.remove(&open.projects, key);
            }
        }
    }
    Ok(())
}
