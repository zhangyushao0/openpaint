//! `Library`: the artwork library on disk, a fjall database (DECISIONS D09, D21, D25),
//! implementing the app's `ProjectStore`. Every call that writes is one atomic batch synced to
//! disk; a save rewrites the head and only the stroke records that changed since the version
//! this library last stored. An older library is upgraded when it is opened.

use std::path::PathBuf;

use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use openpaint_app::{
    PortError, Project, ProjectId, ProjectStore, ProjectSummary, Thumbnail, Title,
};
use openpaint_core::Artwork;

use crate::migrate::upgrade_1_to_2;
use crate::records::{
    LIBRARY, NEXT, PROJECTS, SCHEMA, SCHEMA_KEY, decode_id, decode_schema, encode_schema, head_key,
};
use crate::{read, write};

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

/// Checks the schema of an opened database: a new one is marked with this build's schema, a
/// schema-1 one is upgraded, a newer one is refused.
fn prepare(open: &Open) -> Result<(), PortError> {
    if let Some(bytes) = open.library.get(SCHEMA_KEY).map_err(device)? {
        return match decode_schema(&bytes)? {
            SCHEMA => Ok(()),
            found if found > SCHEMA => Err(PortError::NewerSchema {
                found,
                newest: SCHEMA,
            }),
            _ => Err(PortError::Corrupt {
                what: "library schema",
            }),
        };
    }
    let mut batch = open.db.batch();
    match open.library.get(NEXT).map_err(device)? {
        None => batch.insert(&open.library, SCHEMA_KEY, encode_schema(SCHEMA)),
        Some(next) => {
            let next = decode_id(&next, "next project id")?;
            let mut heads = Vec::new();
            for id in (1..next).map(ProjectId::new) {
                if let Some(head) = open.projects.get(head_key(id)).map_err(device)? {
                    heads.push((id, head.to_vec()));
                }
            }
            let upgrade = upgrade_1_to_2(&heads);
            for (key, value) in upgrade.library {
                batch.insert(&open.library, key, value);
            }
            for (key, value) in upgrade.projects {
                batch.insert(&open.projects, key, value);
            }
            log::info!("library upgraded from schema 1 ({} projects)", heads.len());
        }
    }
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
}
