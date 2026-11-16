//! The blind pairs answered (S147, `openpaint_practice::BlindPair`) in the library's record space
//! `PAIRS` (`b`; the store keeps `c` for its change counter), beside the sheets' (`p`, `g`) and the
//! Pomodoro's (`t`): additive, nothing stored before changes shape, and a library with none reads
//! as before. Each record is read only in this shape, every field present; anything else is
//! refused (left out, logged).
//!
//! - `['b'][asked u64][newer's sheet u64][newer's exercise u16]`, big-endian (a re-test's end asks
//!   several within a second; a newer copy is in one pair only, so the key is unique): the tag
//!   `PAIR` (u8), the lesson that asked it (its sheet u64, its index u16), the older copy (its sheet
//!   u64, its exercise's index on that sheet u16), which was shown on the left (u8: 1 the older, 2
//!   the newer) and the answer (u8: 1 left better, 2 alike, 3 right better). 23 bytes.
//!
//! One writer: the lesson's end, on the answer, one write (`put_pair`). The test chore's end
//! removes the pairs asked at the test sheets it removes (`forget_pairs`).

use openpaint_app::{PortError, ProjectStore, Record, Space};
use openpaint_practice::{Answer, BlindPair, CopyRef};
use openpaint_store::{Reader, Writer};

/// The blind pairs' space in the library: the first byte of each of their keys.
pub const PAIRS: Space = Space::new(b'b');
/// The first byte of each pair's bytes.
const PAIR: u8 = 0x50;
/// A pair's bytes.
const PAIR_BYTES: usize = 1 + 8 + 2 + 8 + 2 + 1 + 1;

/// Every pair stored, by the key's order (when answered); one that cannot be read is left out.
///
/// # Errors
/// Whatever reading the library's records fails with.
pub fn pairs(store: &mut dyn ProjectStore) -> Result<Vec<BlindPair>, PortError> {
    let mut pairs = Vec::new();
    for (key, value) in store.records(PAIRS, &[])? {
        match decode(&key, &value) {
            Ok(pair) => pairs.push(pair),
            Err(error) => log::warn!("a blind pair is not read: {error}"),
        }
    }
    pairs.sort_by_key(|pair| (pair.asked, pair.newer));
    Ok(pairs)
}

/// `pair` stored, in one write.
///
/// # Errors
/// Whatever the write fails with; nothing is written then.
pub fn put_pair(store: &mut dyn ProjectStore, pair: &BlindPair) -> Result<(), PortError> {
    let record: Record = (key(pair), encode(pair));
    store.put_records(&[(PAIRS, record)])?;
    log::info!(
        "a blind pair answered at practice sheet {} lesson {}",
        pair.lesson.0,
        pair.lesson.1
    );
    Ok(())
}

/// For debug builds' test chore only (`practice-test end`): every pair asked at one of `sheets`
/// (by id) removed in one write; every other stays. Returns how many went.
///
/// # Errors
/// Whatever reading or the write fails with; nothing is removed then.
pub fn forget_pairs(store: &mut dyn ProjectStore, sheets: &[u64]) -> Result<usize, PortError> {
    let gone: Vec<(Space, Vec<u8>)> = (pairs(store)?.iter())
        .filter(|pair| sheets.contains(&pair.lesson.0))
        .map(|pair| (PAIRS, key(pair)))
        .collect();
    if !gone.is_empty() {
        store.rewrite_records(&[], &gone)?;
    }
    Ok(gone.len())
}

fn key(pair: &BlindPair) -> Vec<u8> {
    let mut key = vec![PAIRS.byte()];
    key.extend_from_slice(&pair.asked.to_be_bytes());
    key.extend_from_slice(&pair.newer.sheet.to_be_bytes());
    key.extend_from_slice(&pair.newer.exercise.to_be_bytes());
    key
}

fn encode(pair: &BlindPair) -> Vec<u8> {
    let mut writer = Writer::with_capacity(PAIR_BYTES);
    writer.u8(PAIR);
    writer.u64(pair.lesson.0);
    writer.u16(pair.lesson.1);
    writer.u64(pair.older.sheet);
    writer.u16(pair.older.exercise);
    writer.u8(if pair.older_left { 1 } else { 2 });
    writer.u8(pair.answer.code());
    writer.finish()
}

fn decode(key: &[u8], bytes: &[u8]) -> Result<BlindPair, PortError> {
    let mut reader = Reader::new(bytes, "blind pair");
    let rest = key.strip_prefix(&[PAIRS.byte()]).ok_or(reader.corrupt())?;
    let [
        a0,
        a1,
        a2,
        a3,
        a4,
        a5,
        a6,
        a7,
        s0,
        s1,
        s2,
        s3,
        s4,
        s5,
        s6,
        s7,
        e0,
        e1,
    ] = *rest
    else {
        return Err(reader.corrupt());
    };
    let asked = u64::from_be_bytes([a0, a1, a2, a3, a4, a5, a6, a7]);
    let sheet = u64::from_be_bytes([s0, s1, s2, s3, s4, s5, s6, s7]);
    let exercise = u16::from_be_bytes([e0, e1]);
    if reader.u8()? != PAIR {
        return Err(reader.corrupt());
    }
    let lesson = (reader.u64()?, reader.u16()?);
    let older = CopyRef {
        sheet: reader.u64()?,
        exercise: reader.u16()?,
    };
    let older_left = match reader.u8()? {
        1 => true,
        2 => false,
        _ => return Err(reader.corrupt()),
    };
    let answer = Answer::of_code(reader.u8()?).ok_or(reader.corrupt())?;
    reader.end()?;
    Ok(BlindPair {
        asked,
        lesson,
        older,
        newer: CopyRef { sheet, exercise },
        older_left,
        answer,
    })
}
