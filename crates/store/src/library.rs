//! `Library`: the artwork library on disk, a fjall database (DECISIONS D09, D21, D25),
//! implementing the app's `ProjectStore`. Every call that writes is one atomic batch synced to
//! disk (`batch.rs`): it lands whole or not at all, also when the process is killed mid-write; a
//! save rewrites the head and only the stroke records that changed since the version this
//! library last stored. An older library is upgraded when it is opened, one artwork at a time
//! (`upgrade.rs`).

use std::path::PathBuf;

use fjall::{Database, Keyspace, KeyspaceCreateOptions};
use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, Thumbnail, Title,
    VersionId,
};
use openpaint_core::Artwork;

use crate::batch::Batch;
use crate::editor::decode_editor;
use crate::records::{LIBRARY, NEXT, PROJECTS, SCHEMA, SCHEMA_KEY, decode_schema, encode_schema};
use crate::{artwork_file, read, upgrade, versions, write};

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
    /// The most fjall held in memory when it was last logged (`note_memory`).
    noted: u64,
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
            noted: 0,
        }
    }

    /// What fjall holds in memory now: its block cache and its memtables (written, not flushed
    /// yet), in bytes; `None` while the database is closed (S51, the harness's long session).
    #[must_use]
    pub fn memory(&self) -> Option<[u64; 2]> {
        let open = self.open.as_ref()?;
        Some([open.db.cache_size(), open.db.write_buffer_size()])
    }

    /// Logs what fjall holds in memory once it passed the most logged by 4 MiB (S51), so a
    /// session on the device tells how much of the heap is the library's.
    fn note_memory(&mut self) {
        let Some([cache, memtables]) = self.memory() else {
            return;
        };
        if cache + memtables >= self.noted + (4 << 20) {
            self.noted = cache + memtables;
            let mb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
            log::info!(
                "the library holds {:.1} MB in memory (most so far): cache {:.1}, memtables {:.1}",
                mb(cache + memtables),
                mb(cache),
                mb(memtables)
            );
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
/// older one is upgraded one artwork at a time (`upgrade.rs`), a newer one is refused.
fn prepare(open: &Open) -> Result<(), PortError> {
    let schema = match open.library.get(SCHEMA_KEY).map_err(device)? {
        Some(bytes) => decode_schema(&bytes)?,
        // Schema 1 had no schema record; an empty database has no project counter either.
        None if open.library.get(NEXT).map_err(device)?.is_some() => 1,
        None => {
            let mut batch = Batch::new(open);
            batch.insert(&open.library, SCHEMA_KEY, encode_schema(SCHEMA));
            return batch.commit();
        }
    };
    match schema {
        SCHEMA => Ok(()),
        found if found > SCHEMA => Err(PortError::NewerSchema {
            found,
            newest: SCHEMA,
        }),
        older if (1..SCHEMA).contains(&older) => upgrade::upgrade(open, older),
        _ => Err(PortError::Corrupt {
            what: "library schema",
        }),
    }
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
        self.note_memory();
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
