//! Record spaces (S110, D112): a mode keeps its records in the `library` keyspace under keys
//! that begin with its space's byte; their keys after it and their bytes are the mode's, and the
//! store never reads them. It refuses a space whose byte begins one of its own keys and a key
//! outside its space, as `Corrupt`. A space's owner upgrades its old records itself (`Upgrade`,
//! registered with `Library::with_upgrade`): each time the library opens, after the format
//! check and before any call, through `Upgrading`, every write of which is one batch.

use openpaint_app::{PortError, ProjectId, Record, Space};
use openpaint_core::Artwork;

use crate::batch::Batch;
use crate::library::{Open, device};
use crate::{read, write};

/// The first bytes of the store's own keys in `library`: `a` catalogue entries, `c` the change
/// counter, `e` the toolbox, `f` the format, `l` the open project, `n` the next project id, `v`
/// kept versions (`records.rs`, `editor.rs`, `versions.rs`). No space takes one.
const RESERVED: &[u8] = b"acefnlv";

/// What a record that cannot be placed is refused as.
const REFUSED: PortError = PortError::Corrupt {
    what: "record space",
};

/// `space` unless the store's own keys take its byte, and `key` unless it lies outside it.
pub(crate) fn checked(space: Space, key: Option<&[u8]>) -> Result<(), PortError> {
    let outside = key.is_some_and(|key| !space.holds(key));
    if RESERVED.contains(&space.byte()) || outside {
        return Err(REFUSED);
    }
    Ok(())
}

/// Every record of `space`, its key and bytes, in key order.
pub(crate) fn records(open: &Open, space: Space) -> Result<Vec<Record>, PortError> {
    checked(space, None)?;
    let mut records = Vec::new();
    for guard in open.library.prefix([space.byte()]) {
        let (key, value) = guard.into_inner().map_err(device)?;
        records.push((key.to_vec(), value.to_vec()));
    }
    Ok(records)
}

/// `records` of `space` written over what their keys held, in one batch; none, nothing written.
pub(crate) fn put(open: &Open, space: Space, records: &[Record]) -> Result<(), PortError> {
    for (key, _) in records {
        checked(space, Some(key))?;
    }
    if records.is_empty() {
        return Ok(());
    }
    let mut batch = Batch::new(open);
    for (key, bytes) in records {
        batch.insert(&open.library, key.as_slice(), bytes.as_slice());
    }
    batch.commit()
}

/// A record space's owner's upgrade of its old records (D112): what it rewrites, it writes
/// through `library`. Its failure fails the library's opening, as the store's own check does.
pub trait Upgrade: Send {
    /// Upgrades what `library` holds of the owner's records.
    ///
    /// # Errors
    /// Whatever reading or writing the library fails with.
    fn upgrade(&self, library: &Upgrading<'_>) -> Result<(), PortError>;
}

/// The library as an upgrade sees it while it opens.
pub struct Upgrading<'a> {
    open: &'a Open,
}

impl<'a> Upgrading<'a> {
    pub(crate) const fn new(open: &'a Open) -> Self {
        Self { open }
    }

    /// Every record of `space`, its key and bytes, in key order.
    ///
    /// # Errors
    /// `Device` when the library cannot be read; `Corrupt` for a space the store's keys take.
    pub fn records(&self, space: Space) -> Result<Vec<Record>, PortError> {
        records(self.open, space)
    }

    /// `records` of `space` written over what their keys held, in one batch.
    ///
    /// # Errors
    /// `Device` when the batch fails (nothing is written); `Corrupt` for a key outside `space`.
    pub fn put(&self, space: Space, records: &[Record]) -> Result<(), PortError> {
        put(self.open, space, records)
    }

    /// Project `id`'s artwork as stored.
    ///
    /// # Errors
    /// `Missing` when there is no such project; `Corrupt` or `Invalid` when it cannot be read.
    pub fn artwork(&self, id: ProjectId) -> Result<Artwork, PortError> {
        read::artwork(&self.open.projects, id)
    }

    /// Project `id`, a record of `space` owns, written anew whole as `artwork` under a new save
    /// id, with that record as `record` (its key and bytes), in one batch; its catalogue entry
    /// and the open project untouched.
    ///
    /// # Errors
    /// `Device` when the batch fails (nothing is written); `Corrupt` for a key outside `space`.
    pub fn replace_owned(
        &self,
        (id, artwork): (ProjectId, &Artwork),
        space: Space,
        record: (&[u8], &[u8]),
    ) -> Result<(), PortError> {
        checked(space, Some(record.0))?;
        write::replaced(self.open, (id, artwork), record)
    }
}

#[cfg(test)]
#[path = "spaces_tests.rs"]
mod tests;
