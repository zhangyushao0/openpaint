//! `Library`: the artwork library on disk, a fjall database (DECISIONS D09, D21, D25),
//! implementing the app's `ProjectStore`. Every call that writes is one atomic batch synced to
//! disk; a save rewrites the head and only the stroke records that changed since the version
//! this library last stored. An older library is upgraded when it is opened.

use std::path::PathBuf;

use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, Thumbnail, Title,
    VersionId,
};
use openpaint_core::Artwork;

use crate::editor::decode_editor;
use crate::fills::fills_prefix;
use crate::records::{
    LIBRARY, NEXT, PROJECTS, SCHEMA, SCHEMA_KEY, decode_id, decode_schema, encode_schema, head_key,
    strokes_prefix,
};
use crate::steps::{Old, run_steps};
use crate::{artwork_file, read, versions, write};

/// The database and its two keyspaces.
pub(crate) struct Open {
    pub(crate) db: Database,
    pub(crate) library: Keyspace,
    pub(crate) projects: Keyspace,
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

pub(crate) fn device(error: fjall::Error) -> PortError {
    PortError::Device {
        detail: error.to_string(),
    }
}

impl Library {
    /// The library kept in `dir`, created on the first call if missing.
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
            let open = Open {
                db,
                library,
                projects,
            };
            prepare(&open)?;
            log::info!("library opened at {}", self.dir.display());
            self.open = Some(open);
        }
        self.open.as_ref().ok_or(PortError::Stopped)
    }

    /// Runs `call` on the open database; any failure closes it and forgets the stored version.
    fn with<T>(
        &mut self,
        call: impl FnOnce(&Open) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        let result = self.opened().and_then(call);
        if result.is_err() {
            self.open = None;
            self.stored = None;
        }
        result
    }
}

/// Checks the schema of an opened database: a new one is marked with this build's schema, an
/// older one is upgraded step by step in one batch, a newer one is refused.
fn prepare(open: &Open) -> Result<(), PortError> {
    let schema = match open.library.get(SCHEMA_KEY).map_err(device)? {
        Some(bytes) => decode_schema(&bytes)?,
        // Schema 1 had no schema record; an empty database has no project counter either.
        None if open.library.get(NEXT).map_err(device)?.is_some() => 1,
        None => {
            let mut batch = open.db.batch();
            batch.insert(&open.library, SCHEMA_KEY, encode_schema(SCHEMA));
            return commit(batch);
        }
    };
    match schema {
        SCHEMA => Ok(()),
        found if found > SCHEMA => Err(PortError::NewerSchema {
            found,
            newest: SCHEMA,
        }),
        1..=16 => upgrade(open, schema),
        _ => Err(PortError::Corrupt {
            what: "library schema",
        }),
    }
}

/// Runs every migration step from schema `from` on over the records as they are and writes the
/// result in one batch.
fn upgrade(open: &Open, from: u32) -> Result<(), PortError> {
    let old = old_records(open)?;
    let (projects, strokes) = (old.heads.len(), old.strokes.len());
    let stepped = run_steps(from, old);
    let mut batch = open.db.batch();
    for (key, value) in stepped.library {
        batch.insert(&open.library, key, value);
    }
    for (key, value) in stepped.projects {
        batch.insert(&open.projects, key, value);
    }
    commit(batch)?;
    log::info!("library upgraded from schema {from} ({projects} projects, {strokes} strokes)");
    Ok(())
}

fn old_records(open: &Open) -> Result<Old, PortError> {
    let next = match open.library.get(NEXT).map_err(device)? {
        Some(next) => decode_id(&next, "next project id")?,
        None => 1,
    };
    let (mut heads, mut strokes, mut fills) = (Vec::new(), Vec::new(), Vec::new());
    for id in (1..next).map(ProjectId::new) {
        if let Some(head) = open.projects.get(head_key(id)).map_err(device)? {
            heads.push((id, head.to_vec()));
        }
        for (prefix, records) in [
            (strokes_prefix(id), &mut strokes),
            (fills_prefix(id), &mut fills),
        ] {
            for guard in open.projects.prefix(prefix) {
                let (key, value) = guard.into_inner().map_err(device)?;
                records.push((key.to_vec(), value.to_vec()));
            }
        }
    }
    let editor = read::editor(open)?;
    Ok(Old {
        heads,
        strokes,
        fills,
        editor,
    })
}

pub(crate) fn commit(batch: fjall::OwnedWriteBatch) -> Result<(), PortError> {
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(device)
}

impl ProjectStore for Library {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        let project = self.with(read::last)?;
        self.stored = project
            .as_ref()
            .map(|project| (project.id, project.artwork.clone()));
        Ok(project)
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        self.with(read::list)
    }

    fn create(&mut self, artwork: &Artwork, title: &Title) -> Result<ProjectId, PortError> {
        let id = self.with(|open| write::create(open, artwork, title))?;
        log::info!("project {} created", id.get());
        self.stored = Some((id, artwork.clone()));
        Ok(id)
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        let project = self.with(|open| {
            let project = read::project(open, id)?;
            write::open(open, id)?;
            Ok(project)
        })?;
        self.stored = Some((id, project.artwork.clone()));
        Ok(project)
    }

    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<(), PortError> {
        let base = self.stored.take().filter(|(stored, _)| *stored == id);
        self.with(|open| write::save(open, id, base.as_ref().map(|(_, base)| base), artwork))?;
        self.stored = Some((id, artwork.clone()));
        Ok(())
    }

    fn save_thumbnail(&mut self, id: ProjectId, thumbnail: &Thumbnail) -> Result<(), PortError> {
        self.with(|open| write::thumbnail(open, id, thumbnail))
    }

    fn close(&mut self) -> Result<(), PortError> {
        self.with(write::close)
    }

    fn rename(&mut self, id: ProjectId, title: &Title) -> Result<(), PortError> {
        self.with(|open| write::rename(open, id, title))
    }

    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError> {
        let copy = self.with(|open| write::duplicate(open, id))?;
        log::info!("project {} duplicated as {}", id.get(), copy.get());
        Ok(copy)
    }

    fn delete(&mut self, id: ProjectId) -> Result<(), PortError> {
        self.with(|open| write::delete(open, id))?;
        if self
            .stored
            .as_ref()
            .is_some_and(|(stored, _)| *stored == id)
        {
            self.stored = None;
        }
        log::info!("project {} deleted", id.get());
        Ok(())
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        // Decoded outside `with`: an unreadable toolbox is no reason to reopen the database.
        let bytes = self.with(read::editor)?;
        bytes.as_deref().map(decode_editor).transpose()
    }

    fn save_editor(&mut self, state: &EditorState) -> Result<(), PortError> {
        self.with(|open| write::editor(open, state))
    }

    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError> {
        let bytes = self.with(|open| artwork_file::of_project(open, id))?;
        log::info!(
            "project {} as an OpenPaint file of {} bytes",
            id.get(),
            bytes.len()
        );
        Ok(bytes)
    }

    fn import_file(&mut self, bytes: &[u8]) -> Result<ProjectId, PortError> {
        // Decoded and checked before the database is touched: a refused file changes nothing.
        let opened = artwork_file::open(bytes)?;
        let id = self.with(|open| write::add(open, &opened, None))?;
        log::info!(
            "an OpenPaint file of {} bytes is project {}",
            bytes.len(),
            id.get()
        );
        Ok(id)
    }

    fn keep_version(&mut self, id: ProjectId, kept_at: u64) -> Result<(), PortError> {
        self.with(|open| versions::keep(open, id, kept_at))
    }

    fn restore_version(
        &mut self,
        id: ProjectId,
        version: VersionId,
        title: &Title,
    ) -> Result<ProjectId, PortError> {
        let bytes = self.with(|open| versions::file(open, id, version))?;
        let opened = artwork_file::open(&bytes)?;
        let copy = self.with(|open| write::add(open, &opened, Some(title)))?;
        log::info!(
            "version {} of project {} restored as {}",
            version.get(),
            id.get(),
            copy.get()
        );
        Ok(copy)
    }
}
