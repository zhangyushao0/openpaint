//! Practice records (format 1, added in 1.1 beside the records before them, whose bytes stay as
//! they were): `['p'][session u64]` in `library` -> one practice session (DECISIONS D98, D99).
//! Its sheet is an ordinary project that `list` leaves out.
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
//!   before ends before them; the store's upgrade (`upgrade`) gives it the spans its compares'
//!   times tell, once, when the library opens: nothing else reads a record without its time.
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
//! x cells); a record holding one as (6, tone) is turned so by the store's upgrade (`upgrade`).

use std::collections::HashSet;

use openpaint_app::{PortError, ProjectId, Title};
use openpaint_practice::{
    Exercise, ExerciseResult, ItemRef, Kind, Metric, Random, ScaleTone, SessionRecord, Span,
};

use crate::batch::Batch;
use crate::codec::{Reader, Writer};
use crate::library::{Open, device};

/// The first byte of every practice record's key.
pub(crate) const PRACTICE: &[u8] = b"p";

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
    let mut key = PRACTICE.to_vec();
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
    writer.finish()
}

/// What the store's upgrade still owes a record read (`upgrade`): its time (one written before
/// 1.7.0 keeps none) and its value scales' cells (one written before 1.15.0 holding a scale).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Owed {
    pub(crate) time: bool,
    pub(crate) scales: bool,
}

/// The session stored under `key` as `bytes`, and what the store's upgrade owes its record.
pub(crate) fn decode_session(key: &[u8], bytes: &[u8]) -> Result<(SessionRecord, Owed), PortError> {
    let mut reader = Reader::new(bytes, "practice session");
    let id = key
        .strip_prefix(PRACTICE)
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

/// The store's upgrade of practice records, when the library opens: every record written before
/// 1.7.0, which keeps no time, is given the spans its compares' times tell
/// (`SessionRecord::time_of_compares`, S89); one holding a value scale as 1.5.0 to 1.14.0 stored
/// it gets the scale's cells in its kind (`older_scale`, S97); each written again as this version
/// writes it, all in one batch. A record that cannot be read is left as it is (`sessions` leaves
/// it out).
pub(crate) fn upgrade(open: &Open) -> Result<(), PortError> {
    let mut batch = Batch::new(open);
    let mut upgraded = 0;
    for guard in open.library.prefix(PRACTICE) {
        let (key, value) = guard.into_inner().map_err(device)?;
        let Ok((mut session, owed)) = decode_session(&key, &value) else {
            continue;
        };
        if owed.time {
            session.time = session.time_of_compares();
        }
        if owed.time || owed.scales {
            batch.insert(&open.library, key, encode_session(&session));
            upgraded += 1;
        }
    }
    if upgraded == 0 {
        return Ok(());
    }
    batch.commit()?;
    log::info!("{upgraded} practice records upgraded (their time, their value scales' cells)");
    Ok(())
}

/// Every practice session, oldest first; a record that cannot be read is left out (logged).
pub(crate) fn sessions(open: &Open) -> Result<Vec<SessionRecord>, PortError> {
    let mut sessions = Vec::new();
    for guard in open.library.prefix(PRACTICE) {
        let (key, value) = guard.into_inner().map_err(device)?;
        match decode_session(&key, &value) {
            Ok((session, _)) => sessions.push(session),
            Err(error) => log::warn!("a practice record is not read: {error}"),
        }
    }
    Ok(sessions)
}

/// The titles of the practice sheets, by project; a sheet whose entry cannot be read is left out
/// (logged).
pub(crate) fn titles(open: &Open) -> Result<Vec<(ProjectId, Title)>, PortError> {
    let mut titles = Vec::new();
    for session in sessions(open)?.iter().filter(|session| session.canvas != 0) {
        let id = ProjectId::new(session.canvas);
        match crate::read::entry(open, id) {
            Ok(entry) => titles.push((id, entry.title)),
            Err(error) => log::warn!("practice sheet {} has no title read: {error}", id.get()),
        }
    }
    Ok(titles)
}

/// The projects holding practice sheets.
pub(crate) fn sheets(open: &Open) -> Result<HashSet<u64>, PortError> {
    Ok(sessions(open)?
        .iter()
        .map(|session| session.canvas)
        .collect())
}

#[cfg(test)]
#[path = "practice_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "practice_upgrade_tests.rs"]
mod upgrade_tests;
