//! The session records as 1.7.0 to 1.21.6 wrote them (format 1), read only to convert them
//! (`mod.rs`): a private copy of that build's decoder, deleted with the conversion (S121).
//!
//! - Session: stage (u8: 1 to 8, 0 a long colour copy, 255 a drill), number in the stage (u32),
//!   seed (u64), its sheet's project (u64, 0 none), started and finished (u64 seconds since 1970
//!   UTC, finished 0 while open), the exercise shown (u32); its exercises, a count (u32) and each
//!   one's kind and variant (u8 each) and seed (u64); its results, a count (u32) and each one's
//!   exercise (u32), when it was first compared (u64) and its metrics, a count (u32) and each
//!   one's code (u8) and value (f64).
//! - Then the results' tail, each result in the same order: how often it was compared (u32) and
//!   the sheet's first stroke id after its first Compare (u64, 0 unknown).
//! - Then how many of the first exercises are the warm-up (u32).
//! - Then the pictures: a count (u32) and each one's exercise (u32), pack id and item id
//!   (strings).
//! - Then the spans: a count (u32) and each one's start and end (u64).
//! - Last its sheet's layout, a byte: 1.

use openpaint_app::PortError;
use openpaint_practice::{Exercise, ItemRef, Kind, Metric, Metrics, Span};
use openpaint_store::Reader;

use crate::records::PRACTICE;

/// The layout an old record ends with (S99).
const LAID: u8 = 1;
const EXERCISE_BYTES: usize = 1 + 1 + 8;
const RESULT_BYTES: usize = 4 + 8 + 4;
const METRIC_BYTES: usize = 1 + 8;
const PICTURE_BYTES: usize = 4 + 4 + 4;
const SPAN_BYTES: usize = 8 + 8;

/// One session record as it was stored.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Old {
    pub id: u64,
    pub stage: u8,
    pub number: u32,
    pub canvas: u64,
    pub started: u64,
    pub finished: Option<u64>,
    pub current: u32,
    pub exercises: Vec<Exercise>,
    pub warm_up: u32,
    pub results: Vec<OldResult>,
    pub time: Vec<Span>,
}

/// One compared exercise as it was stored.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct OldResult {
    pub index: u32,
    pub checked_at: u64,
    pub compares: u32,
    pub corrected_from: Option<u64>,
    pub metrics: Metrics,
}

/// The session stored under `key` as `bytes`.
pub(super) fn decode_old(key: &[u8], bytes: &[u8]) -> Result<Old, PortError> {
    let mut reader = Reader::new(bytes, "practice session");
    let id = key
        .strip_prefix(&[PRACTICE.byte()])
        .and_then(|id| id.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or(reader.corrupt())?;
    let (stage, number, _seed) = (reader.u8()?, reader.u32()?, reader.u64()?);
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
    Ok(Old {
        id,
        stage,
        number,
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

fn decode_results(reader: &mut Reader<'_>) -> Result<Vec<OldResult>, PortError> {
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
        results.push(OldResult {
            index,
            checked_at,
            compares: 0,
            corrected_from: None,
            metrics,
        });
    }
    Ok(results)
}

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
