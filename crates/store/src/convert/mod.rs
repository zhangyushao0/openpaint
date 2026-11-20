//! Format 1 to format 2 (DECISIONS D125), once: S163's build converts a format-1 library at its
//! first opening (`library::prepare`) and a format-1 OpenPaint file as it is imported; the next
//! slice deletes this step, its tests and the format-1 sample.
//!
//! What changes: each stroke record gains its start after its id (0, unknown); each edit record
//! its place after how many of its slot's strokes lie under it (after k > 0 strokes the slot's
//! k-th stroke's id plus one, after none the slot's first stroke's id, else the head's next stroke
//! id); each kept version, an OpenPaint file, alike, marked format 2 with its sum made anew; then
//! the library's `format` 2. Nothing else changes. Every converted record is checked to be the
//! old bytes with the field inserted and to read as format 2 before the one batch commits: the
//! library becomes format 2 whole, or stays format 1 with nothing written.

use std::borrow::Cow;
use std::collections::HashMap;

use openpaint_app::PortError;
use openpaint_core::EditParts;

use crate::artwork_file::MAGIC;
use crate::batch::Batch;
use crate::codec::{Reader, Writer};
use crate::edits::decode_record;
use crate::library::{Open, device};
use crate::records::{FORMAT, FORMAT_KEY, NEXT_STROKE_AT, encode_format};
use crate::strokes::decode_stroke;

/// The format converted.
pub(crate) const FROM: u32 = 1;
/// Where a stroke record's start goes: after its id.
const STROKE_AT: usize = 8;
/// Where an edit record's place goes: after its kind and how many strokes lie under it.
const EDIT_AT: usize = 1 + 4;

/// Records as one project holds them: keys without the project's id, and values.
type Records = Vec<(Vec<u8>, Vec<u8>)>;

/// The format-1 library `open` made format 2 in one batch, or left as it was on any failure.
pub(crate) fn library(open: &Open) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    let (mut converted, mut current, mut held) = (0, None, Records::new());
    for guard in open.projects.iter() {
        let (key, value) = guard.into_inner().map_err(device)?;
        let Some((id, rest)) = key.split_first_chunk::<8>() else {
            return Err(corrupt());
        };
        if current != Some(*id) {
            if let Some(done) = current {
                converted += write(open, &mut batch, done, &held)?;
            }
            (current, held) = (Some(*id), Vec::new());
        }
        held.push((rest.to_vec(), value.to_vec()));
    }
    if let Some(done) = current {
        converted += write(open, &mut batch, done, &held)?;
    }
    batch.insert(&open.library, FORMAT_KEY, encode_format());
    batch.commit()?;
    log::info!(
        "the library is format {FORMAT} now: {converted} records of format {FROM} converted"
    );
    Ok(())
}

/// `bytes` as this build reads OpenPaint files: one of format 1 converted (`file`), any other as
/// it is.
pub(crate) fn upgraded(bytes: &[u8]) -> Result<Cow<'_, [u8]>, PortError> {
    let format = (bytes.strip_prefix(&MAGIC[..]))
        .and_then(|body| body.first_chunk::<4>())
        .map(|format| u32::from_le_bytes(*format));
    match format {
        Some(FROM) => file(bytes).map(Cow::Owned),
        _ => Ok(Cow::Borrowed(bytes)),
    }
}

/// A format-1 OpenPaint file (a kept version, a file imported) as format 2: its records converted
/// as a project's, its title as it was, its sum made anew.
fn file(bytes: &[u8]) -> Result<Vec<u8>, PortError> {
    let damaged = PortError::Corrupt {
        what: "OpenPaint file",
    };
    let (content, sum) = bytes.split_last_chunk::<4>().ok_or(damaged.clone())?;
    if crc32fast::hash(content) != u32::from_le_bytes(*sum) {
        return Err(damaged);
    }
    let body = content.strip_prefix(&MAGIC[..]).ok_or(damaged.clone())?;
    let mut reader = Reader::new(body, "OpenPaint file");
    if reader.u32()? != FROM {
        return Err(damaged);
    }
    let title = reader.blob()?;
    let count = reader.count(8)?;
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        records.push((reader.blob()?.to_vec(), reader.blob()?.to_vec()));
    }
    reader.end()?;
    for (at, value) in project(&records)? {
        records[at].1 = value;
    }
    let mut writer = Writer::with_capacity(bytes.len() + 8 * records.len());
    writer.bytes(MAGIC);
    writer.u32(FORMAT);
    writer.blob(title);
    writer.len(records.len());
    for (key, value) in &records {
        writer.blob(key);
        writer.blob(value);
    }
    let mut converted = writer.finish();
    let sum = crc32fast::hash(&converted);
    converted.extend_from_slice(&sum.to_le_bytes());
    Ok(converted)
}

/// Project `id`'s records `held` converted into `batch`; how many.
fn write(open: &Open, batch: &mut Batch, id: [u8; 8], held: &Records) -> Result<usize, PortError> {
    let converted = project(held)?;
    let count = converted.len();
    for (at, value) in converted {
        batch.insert(&open.projects, [&id[..], &held[at].0].concat(), value);
    }
    Ok(count)
}

/// One project's records (keys without its id, in key order) as format 2: each record that
/// changes, by its index among them, converted and checked.
fn project(records: &[(Vec<u8>, Vec<u8>)]) -> Result<Vec<(usize, Vec<u8>)>, PortError> {
    let (mut ids, mut next) = (HashMap::new(), 0);
    for (key, value) in records {
        match key.split_first() {
            Some((b'h', [])) => next = u64_at(value, NEXT_STROKE_AT)?,
            Some((b's', place)) => {
                ids.insert(slot_and_index(place)?, u64_at(value, 0)?);
            }
            _ => {}
        }
    }
    let mut converted = Vec::new();
    for (at, (key, value)) in records.iter().enumerate() {
        let value = match key.split_first() {
            Some((b's', _)) => stroke(value)?,
            Some((b'e', place)) => {
                let (slot, _) = slot_and_index(place)?;
                edit(value, |after| {
                    let stroke = match after.checked_sub(1) {
                        Some(last) => ids.get(&(slot, last)).map(|id| id + 1),
                        None => ids.get(&(slot, 0)).copied(),
                    };
                    stroke.unwrap_or(next)
                })?
            }
            Some((b'v', _)) => file(value)?,
            _ => continue,
        };
        converted.push((at, value));
    }
    Ok(converted)
}

/// A stroke record with its start, unknown, after its id.
fn stroke(old: &[u8]) -> Result<Vec<u8>, PortError> {
    let new = inserted(old, STROKE_AT, 0)?;
    match decode_stroke(&new) {
        Ok(parts) if parts.began_ms == 0 => Ok(new),
        _ => Err(corrupt()),
    }
}

/// An edit record with the place `place` gives for how many strokes lie under it.
fn edit(old: &[u8], place: impl FnOnce(u64) -> u64) -> Result<Vec<u8>, PortError> {
    let after = old
        .get(1..EDIT_AT)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok());
    let after = u64::from(u32::from_le_bytes(after.ok_or_else(corrupt)?));
    let place = place(after);
    let new = inserted(old, EDIT_AT, place)?;
    match decode_record(&new) {
        Ok((parts, _)) if placed(&parts) == (after, place) => Ok(new),
        _ => Err(corrupt()),
    }
}

/// An edit's `after` and its place, as format 2 read them.
const fn placed(edit: &EditParts) -> (u64, u64) {
    match edit {
        EditParts::Fill(fill) => (fill.after, fill.before_stroke),
        EditParts::Transform(transform) => (transform.after, transform.before_stroke),
        EditParts::Adjust(adjust) => (adjust.after, adjust.before_stroke),
        EditParts::Nested(nested) => (nested.after, nested.before_stroke),
        EditParts::Clear(clear) => (clear.after, clear.before_stroke),
    }
}

/// `old` with `field` inserted at `at`, checked to be exactly the old bytes and the field.
fn inserted(old: &[u8], at: usize, field: u64) -> Result<Vec<u8>, PortError> {
    let (Some(before), Some(after)) = (old.get(..at), old.get(at..)) else {
        return Err(corrupt());
    };
    let new = [before, &field.to_le_bytes()[..], after].concat();
    let exact = new.len() == old.len() + 8
        && new[..at] == old[..at]
        && new[at..at + 8] == field.to_le_bytes()
        && new[at + 8..] == old[at..];
    if exact { Ok(new) } else { Err(corrupt()) }
}

/// The little-endian u64 at `at` in `bytes`.
fn u64_at(bytes: &[u8], at: usize) -> Result<u64, PortError> {
    let value = bytes.get(at..).and_then(|rest| rest.first_chunk::<8>());
    value
        .map(|value| u64::from_le_bytes(*value))
        .ok_or_else(corrupt)
}

/// A stroke's or an edit's key after its kind: its slot and its index (big-endian).
fn slot_and_index(key: &[u8]) -> Result<(u64, u64), PortError> {
    let (slot, index) = key.split_first_chunk::<8>().ok_or_else(corrupt)?;
    let index: [u8; 8] = index.try_into().map_err(|_| corrupt())?;
    Ok((u64::from_be_bytes(*slot), u64::from_be_bytes(index)))
}

fn corrupt() -> PortError {
    log::error!("a format-1 record could not be converted; the library is left as it was");
    PortError::Corrupt {
        what: "format-1 record",
    }
}

#[cfg(test)]
mod tests;
