//! Practice records (format 1): `['p'][session u64]` in the library's record space `PRACTICE`
//! -> one practice session (DECISIONS D98, D99, D112). Its sheet is an ordinary project the record
//! owns, which the gallery never lists. A record is read only in this shape, every field present;
//! anything else is refused (`sessions` leaves it out).
//!
//! - Session: stage (u8), number in the stage (u32), seed (u64), its sheet's project (u64, 0
//!   none), started and finished (u64 seconds since 1970 UTC, finished 0 while open), the
//!   exercise shown (u32); its exercises, a count (u32) and each one's kind and variant (u8 each,
//!   `Kind::code`) and seed (u64); its results, a count (u32) and each one's exercise (u32), when
//!   it was first compared (u64) and its metrics, a count (u32) and each one's code (u8,
//!   `Metric::code`) and value (f64). A kind or a metric of a code this version does not know is
//!   refused (retired codes are never reused).
//! - Then the results' tail, each result in the same order: how often it was compared (u32) and
//!   the sheet's first stroke id after its first Compare (u64, 0 unknown).
//! - Then how many of the first exercises are the warm-up (u32).
//! - Then the pictures single shapes and colour copies copy: a count (u32) and each one's
//!   exercise (u32) and picture, its pack's id and its item's id (strings).
//! - Then the spans its sheet was practised in (S89): a count (u32) and each one's start and end
//!   (u64 seconds since 1970 UTC, the end not before the start).
//! - Last its sheet's layout, a byte: 1, its pictures' frames their shape (`LAID`, S99).
//!
//! A long colour copy is a record of stage 0 (`PROJECT`) holding its one exercise and its picture
//! (D105), its name its sheet's title; a drill (S91) a record of stage `DRILL` (255), its number
//! among its kind's drills, its exercises all of the kind it drills. A value scale's kind holds
//! its cells, (6, tone + 2 x cells).

use openpaint_app::{PortError, Space};
use openpaint_practice::{Exercise, ExerciseResult, ItemRef, Kind, Metric, SessionRecord, Span};
use openpaint_store::{Reader, Writer};

/// The practice records' space in the library: the first byte of every practice record's key.
pub const PRACTICE: Space = Space::new(b'p');
/// The layout a record ends with (S99): its pictures' frames their shape.
const LAID: u8 = 1;

/// Bytes of an exercise, and of a result and a metric without their lists, and of a result's
/// tail, a picture's place and a span.
const EXERCISE_BYTES: usize = 1 + 1 + 8;
const RESULT_BYTES: usize = 4 + 8 + 4;
const METRIC_BYTES: usize = 1 + 8;
const TAIL_BYTES: usize = 4 + 8;
const PICTURE_BYTES: usize = 4 + 4 + 4;
const SPAN_BYTES: usize = 8 + 8;

/// The key of session `id`'s record.
pub(crate) fn practice_key(id: u64) -> Vec<u8> {
    let mut key = vec![PRACTICE.byte()];
    key.extend_from_slice(&id.to_be_bytes());
    key
}

pub(crate) fn encode_session(session: &SessionRecord) -> Vec<u8> {
    let results = session.results.len() * (RESULT_BYTES + TAIL_BYTES);
    let spans = session.time.len() * SPAN_BYTES;
    let mut writer =
        Writer::with_capacity(72 + session.exercises.len() * EXERCISE_BYTES + results + spans);
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
    for result in &session.results {
        writer.u32(result.compares);
        writer.u64(result.corrected_from.unwrap_or(0));
    }
    writer.u32(session.warm_up);
    let pictures: Vec<(usize, &ItemRef)> = (session.exercises.iter().enumerate())
        .filter_map(|(index, exercise)| Some((index, exercise.picture.as_ref()?)))
        .collect();
    writer.len(pictures.len());
    for (index, picture) in pictures {
        writer.len(index);
        writer.str(&picture.pack);
        writer.str(&picture.item);
    }
    writer.len(session.time.len());
    for span in &session.time {
        writer.u64(span.from);
        writer.u64(span.to);
    }
    writer.u8(LAID);
    writer.finish()
}

/// The session stored under `key` as `bytes`.
pub(crate) fn decode_session(key: &[u8], bytes: &[u8]) -> Result<SessionRecord, PortError> {
    let mut reader = Reader::new(bytes, "practice session");
    let id = key
        .strip_prefix(&[PRACTICE.byte()])
        .and_then(|id| id.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or(reader.corrupt())?;
    let (stage, number, seed) = (reader.u8()?, reader.u32()?, reader.u64()?);
    let (canvas, started, finished) = (reader.u64()?, reader.u64()?, reader.u64()?);
    let current = reader.u32()?;
    let mut exercises = decode_exercises(&mut reader)?;
    let mut results = decode_results(&mut reader)?;
    for result in &mut results {
        result.compares = reader.u32()?;
        result.corrected_from = Some(reader.u64()?).filter(|id| *id != 0);
    }
    let warm_up = reader.u32()?;
    decode_pictures(&mut reader, &mut exercises)?;
    let time = decode_time(&mut reader)?;
    if reader.u8()? != LAID {
        return Err(reader.corrupt());
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
        warm_up,
        results,
        time,
    })
}

/// The exercises; a kind this version does not know is refused.
fn decode_exercises(reader: &mut Reader<'_>) -> Result<Vec<Exercise>, PortError> {
    let count = reader.count(EXERCISE_BYTES)?;
    let mut exercises = Vec::with_capacity(count);
    for _ in 0..count {
        let (kind, variant, seed) = (reader.u8()?, reader.u8()?, reader.u64()?);
        let kind = Kind::of_code(kind, variant).ok_or(reader.corrupt())?;
        exercises.push(Exercise::generated(kind, seed));
    }
    Ok(exercises)
}

/// The results without their tail (`decode_session` reads it after them); a metric this version
/// does not know is refused.
fn decode_results(reader: &mut Reader<'_>) -> Result<Vec<ExerciseResult>, PortError> {
    let count = reader.count(RESULT_BYTES)?;
    let mut results = Vec::with_capacity(count);
    for _ in 0..count {
        let (index, checked_at) = (reader.u32()?, reader.u64()?);
        let count = reader.count(METRIC_BYTES)?;
        let mut metrics = Vec::with_capacity(count);
        for _ in 0..count {
            let (code, value) = (reader.u8()?, reader.f64()?);
            let metric = Metric::of_code(code).ok_or(reader.corrupt())?;
            metrics.push((metric, value));
        }
        results.push(ExerciseResult {
            index,
            checked_at,
            compares: 0,
            corrected_from: None,
            metrics,
        });
    }
    Ok(results)
}

/// The pictures of `exercises`.
fn decode_pictures(reader: &mut Reader<'_>, exercises: &mut [Exercise]) -> Result<(), PortError> {
    for _ in 0..reader.count(PICTURE_BYTES)? {
        let index = reader.u32()? as usize;
        let picture = ItemRef {
            pack: reader.str()?,
            item: reader.str()?,
        };
        let exercise = exercises.get_mut(index).ok_or(reader.corrupt())?;
        exercise.picture = Some(picture);
    }
    Ok(())
}

/// The spans the sheet was practised in; one ending before it starts is refused.
fn decode_time(reader: &mut Reader<'_>) -> Result<Vec<Span>, PortError> {
    let count = reader.count(SPAN_BYTES)?;
    let mut time = Vec::with_capacity(count);
    for _ in 0..count {
        let span = Span {
            from: reader.u64()?,
            to: reader.u64()?,
        };
        if span.to < span.from {
            return Err(reader.corrupt());
        }
        time.push(span);
    }
    Ok(time)
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
