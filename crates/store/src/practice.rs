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
//! - Then (1.4.0 on, only when it holds any) the pictures single shapes copy: a count (u32) and
//!   each one's exercise (u32) and picture, its pack's id and its item's id (strings). A record
//!   without ends before them, as 1.3.0 wrote it.
//!
//! 1.5.0 adds no field: kinds 5 to 7 (colour combinations, value scales, a colour copy), metric
//! codes 29 to 34, and a long colour copy as a record of stage 0 (`PROJECT`) holding its one
//! exercise and its picture (D105). 1.5.1 adds none either: a value scale's mean off its even
//! steps in lightness is code 35 (32, on the wheel's value, retired); a colour copy's name is its
//! sheet's title. 1.6.0 adds none: kinds 8 to 10 (a box to complete, free boxes, an ellipse),
//! which keep no numbers. 1.6.1 adds none: a colour's lightness, chroma and hue parts of its
//! Oklab difference are codes 36 to 41 (21 to 23 and 25 to 27, on the wheel's HSV, retired).

use std::collections::HashSet;

use openpaint_app::{PortError, ProjectId, Title};
use openpaint_practice::{Exercise, ExerciseResult, ItemRef, Kind, Metric, SessionRecord};

use crate::codec::{Reader, Writer};
use crate::library::{Open, device};

/// The first byte of every practice record's key.
pub(crate) const PRACTICE: &[u8] = b"p";

/// Bytes of an exercise, and of a result and a metric without their lists, and of a result's
/// tail.
const EXERCISE_BYTES: usize = 1 + 1 + 8;
const RESULT_BYTES: usize = 4 + 8 + 4;
const METRIC_BYTES: usize = 1 + 8;
const TAIL_BYTES: usize = 4 + 8;
const PICTURE_BYTES: usize = 4 + 4 + 4;

/// The key of session `id`'s record.
pub(crate) fn practice_key(id: u64) -> Vec<u8> {
    let mut key = PRACTICE.to_vec();
    key.extend_from_slice(&id.to_be_bytes());
    key
}

pub(crate) fn encode_session(session: &SessionRecord) -> Vec<u8> {
    let results = session.results.len() * (RESULT_BYTES + TAIL_BYTES);
    let mut writer = Writer::with_capacity(64 + session.exercises.len() * EXERCISE_BYTES + results);
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
    // A session without pictures ends here, as 1.3.0 wrote it.
    if pictures.is_empty() {
        return writer.finish();
    }
    writer.len(pictures.len());
    for (index, picture) in pictures {
        writer.len(index);
        writer.str(&picture.pack);
        writer.str(&picture.item);
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
        exercises.push(Exercise::generated(kind, reader.u64()?));
    }
    let mut results = decode_results(&mut reader)?;
    if !reader.is_done() {
        for result in &mut results {
            result.compares = reader.u32()?;
            result.corrected_from = Some(reader.u64()?).filter(|id| *id != 0);
        }
    }
    let warm_up = if reader.is_done() { 0 } else { reader.u32()? };
    if !reader.is_done() {
        for _ in 0..reader.count(PICTURE_BYTES)? {
            let index = reader.u32()? as usize;
            let picture = ItemRef {
                pack: reader.str()?,
                item: reader.str()?,
            };
            let exercise = exercises.get_mut(index).ok_or(reader.corrupt())?;
            exercise.picture = Some(picture);
        }
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
    })
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
mod tests {
    use openpaint_practice::{Combination, LineTier};

    use super::*;

    /// Bytes of the warm-up's count at the record's end.
    const WARM_UP_BYTES: usize = 4;

    fn session() -> SessionRecord {
        let met = [Kind::Line(LineTier::Arc), Kind::Combined(Combination::Near)];
        let mut session = SessionRecord::stage_one(7, 3, 0x5EED_0045, 1_791_201_600, &met);
        assert_eq!(session.warm_up, 4);
        session.canvas = 12;
        session.current = 11;
        session.finished = Some(1_791_202_000);
        let mut line = ExerciseResult::first(4, 1_791_201_700, 31, Vec::new());
        line.compares = 3;
        session.results.push(line);
        session.results.push(ExerciseResult::first(
            14,
            1_791_201_800,
            40,
            vec![(Metric::FinalDelta, 0.031_25), (Metric::Corrections, 2.0)],
        ));
        session
    }

    #[test]
    fn a_session_round_trips_and_damage_is_refused() {
        let session = session();
        let key = practice_key(7);
        let bytes = encode_session(&session);
        assert_eq!(decode_session(&key, &bytes), Ok(session.clone()));
        assert!(decode_session(&key, &bytes[..bytes.len() - 1]).is_err());
        assert!(
            decode_session(&key, &[bytes.as_slice(), &[0]].concat()).is_err(),
            "a byte after the tail"
        );
        assert!(
            decode_session(b"p", &bytes).is_err(),
            "a key without its id"
        );
        let mut unknown = bytes.clone();
        // The first exercise's kind, after the session's fixed fields and the count: none of 1.6.0's.
        unknown[1 + 4 + 8 * 4 + 4 + 4] = 11;
        assert!(decode_session(&key, &unknown).is_err());
    }

    #[test]
    fn a_stage_two_record_keeps_its_pictures_and_one_without_stays_as_1_3_0_wrote_it() {
        let mut session = session();
        let without = encode_session(&session);
        // As 1.3.0 wrote it: the warm-up's count last.
        assert_eq!(without[without.len() - 4..], 4u32.to_le_bytes());
        session.stage = 2;
        session.exercises[1] = Exercise {
            picture: Some(ItemRef {
                pack: "openpaint-shapes".to_owned(),
                item: "teapot-bail".to_owned(),
            }),
            ..Exercise::generated(Kind::Shape(openpaint_practice::Tier::Simple), 9)
        };
        session.exercises[2].picture = Some(ItemRef {
            pack: "personal".to_owned(),
            item: "mine-3".to_owned(),
        });
        session.exercises[2].kind = Kind::Shape(openpaint_practice::Tier::Medium);
        let bytes = encode_session(&session);
        assert_eq!(bytes[..without.len()].len(), without.len());
        assert_eq!(
            decode_session(&practice_key(7), &bytes),
            Ok(session.clone())
        );
        assert!(decode_session(&practice_key(7), &bytes[..bytes.len() - 1]).is_err());
        let mut far = bytes.clone();
        // The first picture's exercise, past the count: one no session holds.
        let at = without.len() + 4;
        far[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
        assert!(decode_session(&practice_key(7), &far).is_err());
    }

    #[test]
    fn week_two_s_colour_work_and_a_long_colour_copy_round_trip() {
        use openpaint_practice::{Arrangement, Plan, ScaleTone};
        let plan = Plan {
            exercises: vec![
                Exercise::generated(Kind::Areas(Arrangement::Blocks), 3),
                Exercise::generated(Kind::Scale(ScaleTone::Hue), 4),
            ],
            warm_up: 0,
        };
        let mut session = SessionRecord::begun(9, (2, 4), (11, 12), plan);
        let areas = [(Metric::AreasDelta, 0.04), (Metric::AreasMadeStrong, 1.0)];
        let scale = [(Metric::ScaleOff, 0.03), (Metric::ScaleReversed, 2.0)];
        (session.results).push(ExerciseResult::first(0, 13, 7, areas.to_vec()));
        (session.results).push(ExerciseResult::first(1, 14, 9, scale.to_vec()));
        let bytes = encode_session(&session);
        assert_eq!(decode_session(&practice_key(9), &bytes), Ok(session));
        let picture = ItemRef {
            pack: "openpaint-shapes".to_owned(),
            item: "teapot-bail".to_owned(),
        };
        let copy = SessionRecord::project(10, 1, (5, 6), picture);
        let read = decode_session(&practice_key(10), &encode_session(&copy));
        assert!(read.as_ref().is_ok_and(SessionRecord::is_project));
        assert_eq!(read, Ok(copy));
    }

    #[test]
    fn boxes_and_ellipses_round_trip_with_no_numbers() {
        use openpaint_practice::{BoxView, Guide, Horizon, Plan};
        let kinds = [
            Kind::Box(BoxView::ThreePoint),
            Kind::FreeBoxes(Horizon::Low),
            Kind::Ellipse(Guide::Plane),
        ];
        let plan = Plan {
            exercises: (kinds.iter()).map(|k| Exercise::generated(*k, 5)).collect(),
            warm_up: 1,
        };
        let mut session = SessionRecord::begun(11, (2, 3), (13, 14), plan);
        (session.results).push(ExerciseResult::first(1, 15, 4, Vec::new()));
        let bytes = encode_session(&session);
        assert_eq!(decode_session(&practice_key(11), &bytes), Ok(session));
    }

    #[test]
    fn a_record_of_1_2_0_reads_without_a_warm_up() {
        // As 1.1.1 to 1.2.0 wrote it: the results' tail, then nothing.
        let session = session();
        let bytes = encode_session(&session);
        let old = &bytes[..bytes.len() - WARM_UP_BYTES];
        let read = decode_session(&practice_key(7), old).expect("read");
        assert_eq!(read.warm_up, 0);
        assert_eq!(read.results, session.results);
        let mut empty = session.clone();
        empty.results.clear();
        let bytes = encode_session(&empty);
        let read = decode_session(&practice_key(7), &bytes[..bytes.len() - WARM_UP_BYTES]);
        assert_eq!(read.map(|r| r.warm_up), Ok(0), "no results, no tail");
        assert_eq!(decode_session(&practice_key(7), &bytes), Ok(empty));
    }

    #[test]
    fn a_record_of_1_1_0_reads_with_its_line_metrics_passed_over() {
        // As 1.1.0 wrote it: no tail, a line's result with the metrics of codes 1 to 8.
        let session = session();
        let bytes = encode_session(&session);
        let mut old = bytes[..bytes.len() - 2 * TAIL_BYTES - WARM_UP_BYTES].to_vec();
        // The line's metric count: before the colour's result (its index, time, count and two
        // metrics).
        let metrics = old.len() - (4 + 8 + 4 + 2 * METRIC_BYTES) - 4;
        assert_eq!(
            old[metrics..metrics + 4],
            0u32.to_le_bytes(),
            "the line's count"
        );
        let mut retired = Writer::with_capacity(4 + 8 * METRIC_BYTES);
        retired.len(8);
        for code in 1..=8 {
            retired.u8(code);
            retired.f64(f64::from(code) / 100.0);
        }
        old.splice(metrics..metrics + 4, retired.finish());
        let read = decode_session(&practice_key(7), &old).expect("read");
        let results: Vec<(u32, u32, Option<u64>, usize)> = read
            .results
            .iter()
            .map(|r| (r.index, r.compares, r.corrected_from, r.metrics.len()))
            .collect();
        assert_eq!(results, vec![(4, 1, None, 0), (14, 1, None, 2)]);
    }
}
