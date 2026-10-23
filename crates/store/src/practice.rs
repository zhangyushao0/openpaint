//! Practice records (format 1, added in 1.1 beside the records before them, whose bytes stay as
//! they were): `['p'][session u64]` in `library` -> one practice session (DECISIONS D98). Its
//! sheet is an ordinary project that `list` leaves out.
//!
//! - Session: stage (u8), number in the stage (u32), seed (u64), its sheet's project (u64, 0
//!   none), started and finished (u64 seconds since 1970 UTC, finished 0 while open), the
//!   exercise shown (u32); its exercises, a count (u32) and each one's kind and variant (u8 each,
//!   `Kind::code`) and seed (u64); its results, a count (u32) and each one's exercise (u32), when
//!   it was checked (u64) and its metrics, a count (u32) and each one's code (u8, `Metric::code`)
//!   and value (f64). A metric of a code this version does not know is passed over.

use std::collections::HashSet;

use openpaint_app::PortError;
use openpaint_practice::{Exercise, ExerciseResult, Kind, Metric, SessionRecord};

use crate::codec::{Reader, Writer};
use crate::library::{Open, device};

/// The first byte of every practice record's key.
pub(crate) const PRACTICE: &[u8] = b"p";

/// Bytes of an exercise, and of a result and a metric without their lists.
const EXERCISE_BYTES: usize = 1 + 1 + 8;
const RESULT_BYTES: usize = 4 + 8 + 4;
const METRIC_BYTES: usize = 1 + 8;

/// The key of session `id`'s record.
pub(crate) fn practice_key(id: u64) -> Vec<u8> {
    let mut key = PRACTICE.to_vec();
    key.extend_from_slice(&id.to_be_bytes());
    key
}

pub(crate) fn encode_session(session: &SessionRecord) -> Vec<u8> {
    let mut writer = Writer::with_capacity(64 + session.exercises.len() * EXERCISE_BYTES);
    writer.u8(session.stage);
    writer.u32(session.number);
    writer.u64(session.seed);
    writer.u64(session.canvas);
    writer.u64(session.started);
    writer.u64(session.finished.unwrap_or(0));
    writer.u32(session.current);
    writer.len(session.exercises.len());
    for exercise in &session.exercises {
        let (kind, variant) = exercise.kind.code();
        writer.u8(kind);
        writer.u8(variant);
        writer.u64(exercise.seed);
    }
    writer.len(session.results.len());
    for result in &session.results {
        writer.u32(result.index);
        writer.u64(result.checked_at);
        writer.len(result.metrics.len());
        for (metric, value) in &result.metrics {
            writer.u8(metric.code());
            writer.f64(*value);
        }
    }
    writer.finish()
}

/// The session stored under `key` as `bytes`.
pub(crate) fn decode_session(key: &[u8], bytes: &[u8]) -> Result<SessionRecord, PortError> {
    let mut reader = Reader::new(bytes, "practice session");
    let id = key
        .strip_prefix(PRACTICE)
        .and_then(|id| id.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or(reader.corrupt())?;
    let (stage, number, seed) = (reader.u8()?, reader.u32()?, reader.u64()?);
    let (canvas, started, finished) = (reader.u64()?, reader.u64()?, reader.u64()?);
    let current = reader.u32()?;
    let count = reader.count(EXERCISE_BYTES)?;
    let mut exercises = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = Kind::of_code(reader.u8()?, reader.u8()?).ok_or(reader.corrupt())?;
        exercises.push(Exercise {
            kind,
            seed: reader.u64()?,
        });
    }
    let count = reader.count(RESULT_BYTES)?;
    let mut results = Vec::with_capacity(count);
    for _ in 0..count {
        let (index, checked_at) = (reader.u32()?, reader.u64()?);
        let count = reader.count(METRIC_BYTES)?;
        let mut metrics = Vec::with_capacity(count);
        for _ in 0..count {
            let (code, value) = (reader.u8()?, reader.f64()?);
            if let Some(metric) = Metric::of_code(code) {
                metrics.push((metric, value));
            }
        }
        results.push(ExerciseResult {
            index,
            checked_at,
            metrics,
        });
    }
    reader.end()?;
    Ok(SessionRecord {
        id,
        stage,
        number,
        seed,
        canvas,
        started,
        finished: (finished != 0).then_some(finished),
        current,
        exercises,
        results,
    })
}

/// Every practice session, oldest first; a record that cannot be read is left out (logged).
pub(crate) fn sessions(open: &Open) -> Result<Vec<SessionRecord>, PortError> {
    let mut sessions = Vec::new();
    for guard in open.library.prefix(PRACTICE) {
        let (key, value) = guard.into_inner().map_err(device)?;
        match decode_session(&key, &value) {
            Ok(session) => sessions.push(session),
            Err(error) => log::warn!("a practice record is not read: {error}"),
        }
    }
    Ok(sessions)
}

/// The projects holding practice sheets.
pub(crate) fn sheets(open: &Open) -> Result<HashSet<u64>, PortError> {
    Ok(sessions(open)?
        .iter()
        .map(|session| session.canvas)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_round_trips_and_damage_is_refused() {
        let mut session = SessionRecord::stage_one(7, 3, 0x5EED_0045, 1_791_201_600);
        session.canvas = 12;
        session.current = 11;
        session.finished = Some(1_791_202_000);
        session.results.push(ExerciseResult {
            index: 4,
            checked_at: 1_791_201_700,
            metrics: vec![(Metric::Frame, 0.031_25), (Metric::Speed, 1.7)],
        });
        let key = practice_key(7);
        let bytes = encode_session(&session);
        assert_eq!(decode_session(&key, &bytes), Ok(session.clone()));
        assert!(decode_session(&key, &bytes[..bytes.len() - 1]).is_err());
        assert!(
            decode_session(b"p", &bytes).is_err(),
            "a key without its id"
        );
        let mut unknown = bytes.clone();
        // The first exercise's kind, after the session's fixed fields and the count.
        unknown[1 + 4 + 8 * 4 + 4 + 4] = 9;
        assert!(decode_session(&key, &unknown).is_err());
    }
}
