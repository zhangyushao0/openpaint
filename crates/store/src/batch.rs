//! Every write to the library goes through one `Batch` (S50, DECISIONS D49): the inserts and
//! removes of one call, across both keyspaces, committed at once and synced to disk. fjall writes
//! a batch to its journal as one run framed by a start marker and an end marker with a check sum,
//! and makes it visible under one sequence number: a reader never sees part of a batch, and a
//! journal cut anywhere inside one (the process killed, the disk full) is dropped when the
//! library next opens. So a save, one batch, lands whole or not at all.
//!
//! The store's own tests make the n-th write of a call fail (`fail_write`), for every n, and
//! check that the library then opens as it was before the call.

use fjall::{Keyspace, OwnedWriteBatch, PersistMode, UserKey, UserValue};
use openpaint_app::PortError;

use crate::library::{Open, device};

/// The writes of one call.
pub(crate) struct Batch {
    inner: OwnedWriteBatch,
    /// A write failed (only ever in the store's tests): the batch is never committed.
    failed: bool,
}

impl Batch {
    pub(crate) fn new(open: &Open) -> Self {
        Self {
            inner: open.db.batch(),
            failed: false,
        }
    }

    pub(crate) fn insert(
        &mut self,
        keyspace: &Keyspace,
        key: impl Into<UserKey>,
        value: impl Into<UserValue>,
    ) {
        self.failed |= faults::fails();
        self.inner.insert(keyspace, key, value);
    }

    pub(crate) fn remove(&mut self, keyspace: &Keyspace, key: impl Into<UserKey>) {
        self.failed |= faults::fails();
        self.inner.remove(keyspace, key);
    }

    /// Writes every insert and remove at once, synced to disk; nothing when any failed.
    pub(crate) fn commit(self) -> Result<(), PortError> {
        if self.failed || faults::fails() {
            return Err(PortError::Device {
                detail: "a write failed".into(),
            });
        }
        self.inner
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(device)
    }
}

/// The store's tests' failing writes. Outside them no write fails here.
#[cfg(not(test))]
mod faults {
    pub(super) const fn fails() -> bool {
        false
    }
}

/// The store's tests' failing writes: each insert, remove and commit counts as one write.
#[cfg(test)]
pub(crate) mod faults {
    use std::cell::Cell;

    thread_local! {
        /// Writes on this thread so far.
        static WRITES: Cell<usize> = const { Cell::new(0) };
        /// The write that fails, counted from the call to `fail_write`.
        static FAILING: Cell<Option<usize>> = const { Cell::new(None) };
    }

    /// The `n`-th write from now on (1 the next) fails, once.
    pub(crate) fn fail_write(n: usize) {
        FAILING.with(|failing| failing.set(Some(WRITES.with(Cell::get) + n)));
    }

    /// Writes on this thread so far.
    pub(crate) fn writes() -> usize {
        WRITES.with(Cell::get)
    }

    pub(super) fn fails() -> bool {
        let count = WRITES.with(|writes| {
            writes.set(writes.get() + 1);
            writes.get()
        });
        let fails = FAILING.with(Cell::get) == Some(count);
        if fails {
            FAILING.with(|failing| failing.set(None));
        }
        fails
    }
}
