//! What `long-session` measures beside the runtime: the library as the app uses it, watched for
//! the versions it keeps (`Watched`), its size on disk, and the process's memory.

use std::path::Path;
use std::sync::{Arc, Mutex};

use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, SaveId, Thumbnail,
    Title, VersionId,
};
use openpaint_core::Artwork;
use openpaint_store::Library;

/// The library, every call passed on; after each version kept, how many the artwork holds, and
/// after each save or version what fjall holds in memory (its cache and memtables).
pub struct Watched {
    pub inner: Library,
    pub versions: Arc<Mutex<usize>>,
    pub fjall: Arc<Mutex<[u64; 2]>>,
}

impl Watched {
    fn saw_memory(&self) {
        if let (Some(memory), Ok(mut seen)) = (self.inner.memory(), self.fjall.lock()) {
            *seen = memory;
        }
    }
}

impl ProjectStore for Watched {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        self.inner.open_last()
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        self.inner.list()
    }

    fn create(&mut self, artwork: &Artwork, title: &Title) -> Result<ProjectId, PortError> {
        self.inner.create(artwork, title)
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        self.inner.load(id)
    }

    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<SaveId, PortError> {
        let saved = self.inner.save(id, artwork)?;
        self.saw_memory();
        Ok(saved)
    }

    fn save_thumbnail(&mut self, id: ProjectId, thumbnail: &Thumbnail) -> Result<(), PortError> {
        self.inner.save_thumbnail(id, thumbnail)
    }

    fn close(&mut self) -> Result<(), PortError> {
        self.inner.close()
    }

    fn rename(&mut self, id: ProjectId, title: &Title) -> Result<(), PortError> {
        self.inner.rename(id, title)
    }

    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError> {
        self.inner.duplicate(id)
    }

    fn delete(&mut self, id: ProjectId) -> Result<(), PortError> {
        self.inner.delete(id)
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        self.inner.editor()
    }

    fn save_editor(&mut self, state: &EditorState) -> Result<(), PortError> {
        self.inner.save_editor(state)
    }

    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError> {
        self.inner.artwork_file(id)
    }

    fn import_file(&mut self, bytes: &[u8]) -> Result<ProjectId, PortError> {
        self.inner.import_file(bytes)
    }

    fn keep_version(&mut self, id: ProjectId, kept_at: u64) -> Result<(), PortError> {
        self.inner.keep_version(id, kept_at)?;
        let listed = self.inner.list()?;
        let kept = listed.iter().find(|item| item.id == id);
        if let (Some(item), Ok(mut versions)) = (kept, self.versions.lock()) {
            *versions = item.versions.len();
        }
        self.saw_memory();
        Ok(())
    }

    fn restore_version(
        &mut self,
        id: ProjectId,
        version: VersionId,
        title: &Title,
    ) -> Result<ProjectId, PortError> {
        self.inner.restore_version(id, version, title)
    }
}

/// Bytes of every file under `dir`.
pub fn size_on_disk(dir: &Path) -> std::io::Result<u64> {
    let mut total = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        total += if kind.is_dir() {
            size_on_disk(&entry.path())?
        } else {
            entry.metadata()?.len()
        };
    }
    Ok(total)
}

/// This process's private memory in bytes (Windows: `Get-Process`'s `PrivateMemorySize64`;
/// elsewhere the resident set from `/proc/self/status`); `None` when it cannot be read.
pub fn process_memory() -> Option<u64> {
    if cfg!(windows) {
        let query = format!(
            "(Get-Process -Id {}).PrivateMemorySize64",
            std::process::id()
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &query])
            .output()
            .ok()?;
        return String::from_utf8_lossy(&output.stdout).trim().parse().ok();
    }
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}
