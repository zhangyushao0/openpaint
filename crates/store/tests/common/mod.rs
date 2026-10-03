//! Helpers shared by the store tests: fresh directories and raw record dumps.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use fjall::{Database, KeyspaceCreateOptions};
use openpaint_app::Title;

/// The keyspaces of schemas 1 to 3, in dump order.
pub const KEYSPACES: [&str; 2] = ["library", "projects"];

pub fn untitled() -> Title {
    Title::untitled()
}

/// A directory that does not exist yet, under cargo's temporary directory for tests.
pub fn fresh_dir(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("store-{name}-{}-{unique}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear an old test directory");
    }
    dir
}

/// Every record of a closed library as `(keyspace, key, value)`, sorted.
pub fn records(dir: &Path) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let db = Database::builder(dir).open().expect("open raw");
    let mut out = Vec::new();
    for name in KEYSPACES {
        let keyspace = db
            .keyspace(name, KeyspaceCreateOptions::default)
            .expect("keyspace");
        for guard in keyspace.iter() {
            let (key, value) = guard.into_inner().expect("record");
            out.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    out
}

/// Writes `records` into a new library directory, bypassing the store.
pub fn write_records(dir: &Path, records: &[(String, Vec<u8>, Vec<u8>)]) {
    let db = Database::builder(dir).open().expect("open raw");
    for (name, key, value) in records {
        let keyspace = db
            .keyspace(name, KeyspaceCreateOptions::default)
            .expect("keyspace");
        keyspace.insert(key.clone(), value.clone()).expect("insert");
    }
    db.persist(fjall::PersistMode::SyncAll).expect("persist");
}
