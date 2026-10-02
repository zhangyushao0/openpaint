//! `Library`: the artwork library on disk, a fjall database (DECISIONS D09, D21), implementing
//! the app's `ProjectStore`. Every save is one atomic batch synced to disk; it rewrites the head
//! and only the stroke records that changed since the version this library last stored.

use std::collections::HashSet;
use std::path::PathBuf;

use fjall::{Database, Keyspace, KeyspaceCreateOptions, OwnedWriteBatch, PersistMode};
use openpaint_app::{PortError, Project, ProjectId, ProjectStore};
use openpaint_core::{Artwork, ArtworkParts, StrokeParts};

use crate::delta::delta;
use crate::records::{
    LAST, LIBRARY, NEXT, PROJECTS, decode_head, decode_id, decode_stroke, encode_head, encode_id,
    encode_stroke, head_key, layer_prefix, stroke_index, stroke_key, strokes_prefix,
};

/// The database and its two keyspaces.
struct Open {
    db: Database,
    library: Keyspace,
    projects: Keyspace,
}

/// The artwork library in one directory. The database opens on the first call, so creating a
/// `Library` costs nothing on the UI thread; after a failed call it is reopened on the next
/// one (a failed write leaves fjall refusing further writes).
pub struct Library {
    dir: PathBuf,
    open: Option<Open>,
    /// The version the disk holds for one project, known exactly; the base for the next save
    /// of that project. Forgotten whenever a call fails, so that save rewrites the project.
    stored: Option<(ProjectId, Artwork)>,
}

fn device(error: fjall::Error) -> PortError {
    PortError::Device {
        detail: error.to_string(),
    }
}

impl Library {
    /// The library kept in `dir`, created on the first save if missing.
    #[must_use]
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            open: None,
            stored: None,
        }
    }

    fn opened(&mut self) -> Result<&Open, PortError> {
        if self.open.is_none() {
            let db = Database::builder(&self.dir).open().map_err(device)?;
            let library = db
                .keyspace(LIBRARY, KeyspaceCreateOptions::default)
                .map_err(device)?;
            let projects = db
                .keyspace(PROJECTS, KeyspaceCreateOptions::default)
                .map_err(device)?;
            log::info!("library opened at {}", self.dir.display());
            self.open = Some(Open {
                db,
                library,
                projects,
            });
        }
        self.open.as_ref().ok_or(PortError::Stopped)
    }

    /// Runs `call` on the open database; any failure closes it and forgets the stored version.
    fn with<T>(
        &mut self,
        call: impl FnOnce(&Open, Option<(ProjectId, Artwork)>) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        let stored = self.stored.take();
        let result = self.opened().and_then(|open| call(open, stored));
        if result.is_err() {
            self.open = None;
        }
        result
    }
}

impl ProjectStore for Library {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        let project = self.with(|open, _| read_last(open))?;
        self.stored = project
            .as_ref()
            .map(|project| (project.id, project.artwork.clone()));
        Ok(project)
    }

    fn create(&mut self, artwork: &Artwork) -> Result<ProjectId, PortError> {
        let id = self.with(|open, _| {
            let next = match open.library.get(NEXT).map_err(device)? {
                Some(bytes) => decode_id(&bytes, "next project id")?,
                None => 1,
            };
            let id = ProjectId::new(next);
            let mut batch = open.db.batch();
            batch.insert(&open.library, NEXT, encode_id(next + 1));
            write_all(open, &mut batch, id, artwork)?;
            commit(open, batch, id, artwork)?;
            Ok(id)
        })?;
        log::info!("project {} created", id.get());
        self.stored = Some((id, artwork.clone()));
        Ok(id)
    }

    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<(), PortError> {
        self.with(|open, stored| {
            let mut batch = open.db.batch();
            match stored.filter(|(stored, _)| *stored == id) {
                Some((_, base)) => write_delta(open, &mut batch, id, &base, artwork),
                None => write_all(open, &mut batch, id, artwork)?,
            }
            commit(open, batch, id, artwork)
        })?;
        self.stored = Some((id, artwork.clone()));
        Ok(())
    }
}

/// Adds the head and `last`, then writes the batch atomically and syncs it to disk.
fn commit(
    open: &Open,
    mut batch: OwnedWriteBatch,
    id: ProjectId,
    artwork: &Artwork,
) -> Result<(), PortError> {
    batch.insert(&open.projects, head_key(id), encode_head(artwork));
    batch.insert(&open.library, LAST, encode_id(id.get()));
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(device)
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
}

/// Every stroke record of `artwork`, and the removal of any other stroke record the project
/// has on disk. For a new project, and for a save whose base is unknown after a failure.
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
    }
    for guard in open.projects.prefix(strokes_prefix(id)) {
        let key = guard.key().map_err(device)?;
        if !written.contains(key.as_ref()) {
            batch.remove(&open.projects, key);
        }
    }
    Ok(())
}

fn read_last(open: &Open) -> Result<Option<Project>, PortError> {
    let Some(last) = open.library.get(LAST).map_err(device)? else {
        return Ok(None);
    };
    let id = ProjectId::new(decode_id(&last, "last project")?);
    let head = open
        .projects
        .get(head_key(id))
        .map_err(device)?
        .ok_or(PortError::Corrupt {
            what: "artwork head",
        })?;
    let head = decode_head(&head)?;
    let mut layers = Vec::with_capacity(head.layers.len());
    for (mut layer, count) in head.layers {
        layer.strokes = read_strokes(open, id, layer.id, count)?;
        layers.push(layer);
    }
    let artwork = Artwork::from_storage(ArtworkParts {
        width: head.width,
        height: head.height,
        layers,
        active: head.active,
        next_layer_id: head.next_layer_id,
        next_stroke_id: head.next_stroke_id,
    })?;
    log::info!("project {} opened", id.get());
    Ok(Some(Project { id, artwork }))
}

/// The `count` stroke records of one layer, which must sit at indices `0..count`.
fn read_strokes(
    open: &Open,
    id: ProjectId,
    layer: u64,
    count: usize,
) -> Result<Vec<StrokeParts>, PortError> {
    let corrupt = PortError::Corrupt {
        what: "stroke records",
    };
    let prefix = layer_prefix(id, layer);
    let mut strokes = Vec::new();
    for (expected, guard) in open.projects.prefix(&prefix).enumerate() {
        let (key, value) = guard.into_inner().map_err(device)?;
        let in_place = key.len() == prefix.len() + 8 && stroke_index(&key) == Some(expected as u64);
        if expected >= count || !in_place {
            return Err(corrupt);
        }
        strokes.push(decode_stroke(&value)?);
    }
    if strokes.len() == count {
        Ok(strokes)
    } else {
        Err(corrupt)
    }
}
