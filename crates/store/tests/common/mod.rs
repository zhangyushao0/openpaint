//! Helpers shared by the store tests: their own directories and raw record dumps.

mod test_dir;

use std::path::Path;

use fjall::{Database, KeyspaceCreateOptions};
use openpaint_app::Title;

pub use test_dir::TestDir;

/// The library's keyspaces, in dump order.
pub const KEYSPACES: [&str; 2] = ["library", "projects"];

pub fn untitled() -> Title {
    Title::untitled()
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
