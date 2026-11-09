//! Record spaces (S110, D112): a mode keeps its records in the `library` keyspace under keys
//! that begin with its space's byte; their keys after it and their bytes are the mode's, and the
//! store never reads them. It refuses a space whose byte begins one of its own keys and a key
//! outside its space, as `Corrupt`.

use openpaint_app::{PortError, Record, Space};

use crate::batch::Batch;
use crate::library::{Open, device};

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

#[cfg(test)]
#[path = "spaces_tests.rs"]
mod tests;
