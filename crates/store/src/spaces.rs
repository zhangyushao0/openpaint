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

/// Every record of `space` whose key goes on from its byte with `under`, its key and bytes, in
/// key order.
pub(crate) fn records(open: &Open, space: Space, under: &[u8]) -> Result<Vec<Record>, PortError> {
    checked(space, None)?;
    let mut prefix = vec![space.byte()];
    prefix.extend_from_slice(under);
    let mut records = Vec::new();
    for guard in open.library.prefix(prefix) {
        let (key, value) = guard.into_inner().map_err(device)?;
        records.push((key.to_vec(), value.to_vec()));
    }
    Ok(records)
}

/// `records` written over what their keys held and the records `gone` removed, in one batch;
/// none of either, nothing written.
pub(crate) fn put(
    open: &Open,
    records: &[(Space, Record)],
    gone: &[(Space, Vec<u8>)],
) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    insert(open, &mut batch, records)?;
    remove(open, &mut batch, gone)?;
    if records.is_empty() && gone.is_empty() {
        return Ok(());
    }
    batch.commit()
}

/// `records` added to `batch` over what their keys held, once every key is checked against its
/// space; one refused, nothing added.
pub(crate) fn insert(
    open: &Open,
    batch: &mut Batch,
    records: &[(Space, Record)],
) -> Result<(), PortError> {
    for (space, (key, _)) in records {
        checked(*space, Some(key))?;
    }
    for (_, (key, bytes)) in records {
        batch.insert(&open.library, key.as_slice(), bytes.as_slice());
    }
    Ok(())
}

/// `keys` removed in `batch`, once every key is checked against its space; one refused, nothing
/// removed.
pub(crate) fn remove(
    open: &Open,
    batch: &mut Batch,
    keys: &[(Space, Vec<u8>)],
) -> Result<(), PortError> {
    for (space, key) in keys {
        checked(*space, Some(key))?;
    }
    for (_, key) in keys {
        batch.remove(&open.library, key.as_slice());
    }
    Ok(())
}

#[cfg(test)]
#[path = "spaces_tests.rs"]
mod tests;
