//! Practice records (format 1, added in 1.1 beside the records before them, whose bytes stay as
//! they were): `['p'][session u64]` in the library's record space `PRACTICE` -> one practice
//! session (DECISIONS D98, D99, D112). Its sheet is an ordinary project the record owns, which
//! the gallery never lists.
//!
//! - Session: stage (u8), number in the stage (u32), seed (u64), its sheet's project (u64, 0
//!   none), started and finished (u64 seconds since 1970 UTC, finished 0 while open), the
//!   exercise shown (u32); its exercises, a count (u32) and each one's kind and variant (u8 each,
//!   `Kind::code`) and seed (u64); its results, a count (u32) and each one's exercise (u32), when
//!   it was first compared (u64) and its metrics, a count (u32) and each one's code (u8,
//!   `Metric::code`) and value (f64). A metric of a code this version does not know is passed
//!   over: 1.1.0's line metrics (codes 1 to 8) read as none.
//! - Then (1.1.1 on) the results' tail, each result in the same order: how often it was compared
//!   (u32) and the sheet's first stroke id after its first Compare (u64, 0 unknown). A record
//!   1.1.0 wrote ends before it: compared once, no stroke a correction.
//! - Then (1.3.0 on) how many of the first exercises are the warm-up (u32). A record written
//!   before ends before it: no warm-up.
//! - Then the pictures single shapes and colour copies copy: a count (u32) and each one's
//!   exercise (u32) and picture, its pack's id and its item's id (strings). From 1.4.0 to 1.6.2 a
//!   record without any ended before the count.
//! - Then (1.7.0 on, S89) the spans its sheet was practised in: a count (u32) and each one's
//!   start and end (u64 seconds since 1970 UTC, the end not before the start). A record written
//!   before ends before them; the records' upgrade (`upgrade.rs`) gives it the spans its
//!   compares' times tell, once, when the library opens: nothing else reads a record without
//!   its time.
//! - Then (1.17.0 on, S99) its sheet's layout, a byte: 1, its pictures' frames their shape
//!   (`LAID`). A record written before ends before it, its copies of pictures laid in square
//!   frames; the records' upgrade lays its sheet anew once (`frames.rs`).
//!
//! 1.5.0 adds no field: kinds 5 to 7 (colour combinations, value scales, a colour copy), metric
//! codes 29 to 34, and a long colour copy as a record of stage 0 (`PROJECT`) holding its one
//! exercise and its picture (D105). 1.5.1 adds none either: a value scale's mean off its even
//! steps in lightness is code 35 (32, on the wheel's value, retired); a colour copy's name is its
//! sheet's title. 1.6.0 adds none: kinds 8 to 10 (a box to complete, free boxes, an ellipse),
//! which keep no numbers. 1.6.1 adds none: a colour's lightness, chroma and hue parts of its
//! Oklab difference are codes 36 to 41 (21 to 23 and 25 to 27, on the wheel's HSV, retired).
//! 1.8.0 adds none: kinds 11 and 12 (an estimate on the flat, a point in space) and their errors,
//! metric codes 42 to 46 (D107). 1.9.0 adds none: a drill (S91) is a record of stage `DRILL`
//! (255), its number among its kind's drills, its exercises all of the kind it drills; earlier
//! records read as they are. 1.15.0 adds none: a value scale's kind holds its cells, (6, tone + 2
//! x cells); a record holding one as (6, tone) is turned so by the records' upgrade.

use openpaint_app::{PortError, Space};
use openpaint_practice::{
    Exercise, ExerciseResult, ItemRef, Kind, Metric, Random, ScaleTone, SessionRecord, Span,
};
use openpaint_store::{Reader, Writer};

/// The practice records' space in the library: the first byte of every practice record's key.
pub const PRACTICE: Space = Space::new(b'p');
/// The layout a record ends with from 1.17.0 (S99): its pictures' frames their shape.
pub(crate) const LAID: u8 = 1;

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

/// What the records' upgrade still owes a record read (`upgrade.rs`): its time (one written before
/// 1.7.0 keeps none), its value scales' cells (one written before 1.15.0 holding a scale) and its
/// sheet laid anew (one written before 1.17.0).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Owed {
    pub(crate) time: bool,
    pub(crate) scales: bool,
    pub(crate) frames: bool,
}

/// The session stored under `key` as `bytes`, and what the records' upgrade owes its record.
pub(crate) fn decode_session(key: &[u8], bytes: &[u8]) -> Result<(SessionRecord, Owed), PortError> {
    let mut reader = Reader::new(bytes, "practice session");
    let id = key
        .strip_prefix(&[PRACTICE.byte()])
        .and_then(|id| id.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or(reader.corrupt())?;
    let (stage, number, seed) = (reader.u8()?, reader.u32()?, reader.u64()?);
    let (canvas, started, finished) = (reader.u64()?, reader.u64()?, reader.u64()?);
    let current = reader.u32()?;
    let (mut exercises, scales) = decode_exercises(&mut reader)?;
    let mut results = decode_results(&mut reader)?;
    if !reader.is_done() {
        for result in &mut results {
            result.compares = reader.u32()?;
            result.corrected_from = Some(reader.u64()?).filter(|id| *id != 0);
        }
    }
    let warm_up = if reader.is_done() { 0 } else { reader.u32()? };
    if !reader.is_done() {
        decode_pictures(&mut reader, &mut exercises)?;
    }
    let timed = !reader.is_done();
    let mut time = Vec::new();
    if timed {
        for _ in 0..reader.count(SPAN_BYTES)? {
            let span = Span {
                from: reader.u64()?,
                to: reader.u64()?,
            };
            if span.to < span.from {
                return Err(reader.corrupt());
            }
            time.push(span);
        }
    }
    let laid = !reader.is_done();
    if laid && reader.u8()? != LAID {
        return Err(reader.corrupt());
    }
    reader.end()?;
    let session = SessionRecord {
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
    };
    let owed = Owed {
        time: !timed,
        scales,
        frames: !laid,
    };
    Ok((session, owed))
}

/// The exercises, and whether any is a value scale as 1.5.0 to 1.14.0 stored it (`older_scale`).
fn decode_exercises(reader: &mut Reader<'_>) -> Result<(Vec<Exercise>, bool), PortError> {
    let count = reader.count(EXERCISE_BYTES)?;
    let mut exercises = Vec::with_capacity(count);
    let mut older = false;
    for _ in 0..count {
        let (kind, variant, seed) = (reader.u8()?, reader.u8()?, reader.u64()?);
        let exercise = match Kind::of_code(kind, variant) {
            Some(kind) => Exercise::generated(kind, seed),
            None => {
                older = true;
                older_scale((kind, variant), seed).ok_or(reader.corrupt())?
            }
        };
        exercises.push(exercise);
    }
    Ok((exercises, older))
}

/// SplitMix64's step (`openpaint_practice::Random`): a seed one draw on.
const ONE_DRAW: u64 = 0x9E37_79B9_7F4A_7C15;

/// A value scale as 1.5.0 to 1.14.0 stored it, kind (6, tone): its cells, 5 to 7, were its seed's
/// first draw and its ends the draws after. As this version keeps it (S97): its cells in its kind
/// and its seed one draw on, so it is the same scale, its strokes where they lay. None for any
/// other kind this version does not know.
fn older_scale((kind, variant): (u8, u8), seed: u64) -> Option<Exercise> {
    let tone = *ScaleTone::ALL
        .get(usize::from(variant))
        .filter(|_| kind == 6)?;
    let mut random = Random::new(seed);
    let cells = 5 + ((random.unit() * 3.0) as u8).min(2);
    Some(Exercise::generated(
        Kind::Scale(tone, cells),
        seed.wrapping_add(ONE_DRAW),
    ))
}

/// The results, as a record of 1.1.0 holds them: compared once, no stroke a correction until
/// the tail says otherwise.
fn decode_results(reader: &mut Reader<'_>) -> Result<Vec<ExerciseResult>, PortError> {
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
            compares: 1,
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

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "records_upgrade_tests.rs"]
mod upgrade_tests;
