//! `Library`: the artwork library on disk, a fjall database (DECISIONS D09, D21, D25),
//! implementing the app's `ProjectStore`. Every call that writes is one atomic batch synced to
//! disk; a save rewrites the head and only the stroke records that changed since the version
//! this library last stored. An older library is upgraded when it is opened.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, Thumbnail, Title,
};
use openpaint_core::Artwork;

use crate::editor::{EDITOR, decode_editor};
use crate::fills::fills_prefix;
use crate::migrate::{upgrade_1_to_2, upgrade_2_to_3, upgrade_3_to_4, upgrade_4_to_5};
use crate::migrate_6::upgrade_5_to_6;
use crate::migrate_7::upgrade_6_to_7;
use crate::migrate_8::upgrade_7_to_8;
use crate::migrate_9::upgrade_8_to_9;
use crate::migrate_10::upgrade_9_to_10;
use crate::migrate_11::upgrade_10_to_11;
use crate::migrate_12::upgrade_11_to_12;
use crate::records::{
    LIBRARY, NEXT, PROJECTS, SCHEMA, SCHEMA_KEY, decode_id, decode_schema, encode_schema, head_key,
    strokes_prefix,
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
        1..=11 => upgrade(open, schema),
        _ => Err(PortError::Corrupt {
            what: "library schema",
        }),
    }
}

/// Runs every migration step from schema `from` on over the records as they are, each step
/// seeing the heads and strokes the one before it wrote, and writes the result in one batch.
fn upgrade(open: &Open, from: u32) -> Result<(), PortError> {
    let (mut heads, mut strokes, fills) = old_records(open)?;
    let mut editor = read::editor(open)?;
    let index: HashMap<Vec<u8>, usize> = strokes
        .iter()
        .enumerate()
        .map(|(at, (key, _))| (key.clone(), at))
        .collect();
    let mut library = BTreeMap::new();
    let mut projects = BTreeMap::new();
    for step in from..SCHEMA {
        let upgrade = match step {
            1 => upgrade_1_to_2(&heads),
            2 => upgrade_2_to_3(&heads, &strokes),
            3 => upgrade_3_to_4(&heads, &strokes, editor.as_deref()),
            4 => upgrade_4_to_5(&heads),
            5 => upgrade_5_to_6(&heads, &strokes, editor.as_deref()),
            6 => upgrade_6_to_7(&heads, &strokes),
            // Only this step reads the fill records, so they are never rewritten twice.
            7 => upgrade_7_to_8(&heads, &fills),
            8 => upgrade_8_to_9(&heads, editor.as_deref()),
            9 => upgrade_9_to_10(&heads),
            10 => upgrade_10_to_11(&heads),
            _ => upgrade_11_to_12(&heads),
        };
        if let Some((_, value)) = upgrade.library.iter().find(|(key, _)| key == EDITOR) {
            editor = Some(value.clone());
        }
        for (key, value) in &upgrade.projects {
            if let Some(at) = index.get(key) {
                strokes[*at].1.clone_from(value);
            } else if let Some((_, head)) = heads.iter_mut().find(|(id, _)| head_key(*id) == *key) {
                head.clone_from(value);
            }
        }
        library.extend(upgrade.library);
        projects.extend(upgrade.projects);
    }
    let mut batch = open.db.batch();
    for (key, value) in library {
        batch.insert(&open.library, key, value);
    }
    for (key, value) in projects {
        batch.insert(&open.projects, key, value);
    }
    commit(batch)?;
    log::info!(
        "library upgraded from schema {from} ({} projects, {} strokes)",
        heads.len(),
        strokes.len()
    );
    Ok(())
}

/// Records by key.
type Records = Vec<(Vec<u8>, Vec<u8>)>;
/// Every project's head, every stroke record and every fill record.
type OldRecords = (Vec<(ProjectId, Vec<u8>)>, Records, Records);

fn old_records(open: &Open) -> Result<OldRecords, PortError> {
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
    Ok((heads, strokes, fills))
}

fn commit(batch: fjall::OwnedWriteBatch) -> Result<(), PortError> {
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
}
