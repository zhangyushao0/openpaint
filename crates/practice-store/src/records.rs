//! Practice records (S120): `['p'][sheet u64]` in the library's record space `PRACTICE` -> one
//! practice sheet (DECISIONS D98, D99, D112), and `['g'][canvas u64][lesson u16]` in `GIVENS` ->
//! what that lesson's exercises printed (`givens/`). A sheet is an ordinary project the record
//! owns, which the gallery never lists. A record is read only in this shape, every field present;
//! anything else is refused (`sheets` leaves it out).
//!
//! - The tag `SHEET` (u8; a record beginning otherwise is refused), its project (u64, 0 none),
//!   when it started (u64 seconds since 1970 UTC), its purpose (u8: 1 the day's practice, 2 a
//!   project (a long colour copy, a drawing from life), 3 a test's, `practice-test`).
//! - The spans it was practised in: a count (u32) and each one's start and end (u64 seconds since
//!   1970 UTC, the end not before the start).
//! - Its lessons, a count (u32, one at least), each: started and finished (u64, finished 0 while
//!   open), how it was picked (u8: 2 a kind chosen; 4 the first lesson, S122; 5 a unit's
//!   re-test, S147, then its unit, u8, and whether it is the unit's end, u8 0 or 1), the
//!   exercise in hand (u16), and its exercises, a count (u32), each: its kind and variant (u8
//!   each, `Kind::code`) and seed (u64), its level (u8), its picture (u8: 0 none, 1 one, then its
//!   pack's id and its item's id, strings), its slot (u16), its picture's size (u16 width and
//!   height; 0 0 square), its first Compare (u8: 0 none, 1 one, then when (u64), the sheet's
//!   first stroke id after it (u64) and its metrics, a count (u32) and each one's code (u8,
//!   `Metric::code`) and value (f64)), how often it was compared (u32), its result (u8: 0 none,
//!   else `Outcome::code`, then who judged, u8: 1 the app, 2 the eye) and the seconds spent on it
//!   (u32).
//!
//! The reader reads what this version writes: a code it does not know (a way picked, a kind, a
//! metric, an outcome) is refused. A value scale's kind holds its cells, (6, tone + 2 x cells).
//! The project a record names is also read alone, from the tag and the field after it
//! (`named_project`), so the practice-only clear finds it in a record it could not decode.

use openpaint_app::{PortError, Space};
use openpaint_practice::{
    Exercise, First, ItemRef, Judge, Kind, Laid, Lesson, Metric, Outcome, Picked, Purpose, Sheet,
    Span,
};
use openpaint_store::{Reader, Writer};

/// The practice records' space in the library: the first byte of every sheet record's key.
pub const PRACTICE: Space = Space::new(b'p');
/// The printed givens' space: the first byte of every lesson's givens' key.
pub const GIVENS: Space = Space::new(b'g');
/// The first byte of every sheet record (S120).
const SHEET: u8 = 0x53;

/// The fewest bytes of a span, a lesson, an exercise and a metric.
const SPAN_BYTES: usize = 8 + 8;
const LESSON_BYTES: usize = 8 + 8 + 1 + 2 + 4;
const LAID_BYTES: usize = 1 + 1 + 8 + 1 + 1 + 2 + 2 + 2 + 1 + 4 + 1 + 4;
const METRIC_BYTES: usize = 1 + 8;

/// The key of sheet `id`'s record.
pub(crate) fn sheet_key(id: u64) -> Vec<u8> {
    let mut key = vec![PRACTICE.byte()];
    key.extend_from_slice(&id.to_be_bytes());
    key
}

/// The key of the givens of lesson `lesson` of the sheet on project `canvas`.
pub(crate) fn givens_key(canvas: u64, lesson: u16) -> Vec<u8> {
    let mut key = vec![GIVENS.byte()];
    key.extend_from_slice(&canvas.to_be_bytes());
    key.extend_from_slice(&lesson.to_be_bytes());
    key
}

pub(crate) fn encode_sheet(sheet: &Sheet) -> Vec<u8> {
    let mut writer = Writer::with_capacity(64 + sheet.spans.len() * SPAN_BYTES);
    writer.u8(SHEET);
    writer.u64(sheet.canvas);
    writer.u64(sheet.started);
    writer.u8(match sheet.purpose {
        Purpose::Day => 1,
        Purpose::Copy => 2,
        Purpose::Test => 3,
    });
    writer.len(sheet.spans.len());
    for span in &sheet.spans {
        writer.u64(span.from);
        writer.u64(span.to);
    }
    writer.len(sheet.lessons.len());
    for lesson in &sheet.lessons {
        writer.u64(lesson.started);
        writer.u64(lesson.finished.unwrap_or(0));
        match lesson.picked {
            Picked::Chosen => writer.u8(2),
            Picked::First => writer.u8(4),
            Picked::Retest { unit, end } => {
                writer.u8(5);
                writer.u8(unit);
                writer.bool(end);
            }
        }
        writer.u16(lesson.at);
        writer.len(lesson.exercises.len());
        for laid in &lesson.exercises {
            write_laid(&mut writer, laid);
        }
    }
    writer.finish()
}

fn write_laid(writer: &mut Writer, laid: &Laid) {
    let (kind, variant) = laid.exercise.kind.code();
    writer.u8(kind);
    writer.u8(variant);
    writer.u64(laid.exercise.seed);
    writer.u8(laid.level);
    match &laid.exercise.picture {
        None => writer.u8(0),
        Some(picture) => {
            writer.u8(1);
            writer.str(&picture.pack);
            writer.str(&picture.item);
        }
    }
    writer.u16(laid.slot);
    writer.u16(laid.frame[0]);
    writer.u16(laid.frame[1]);
    match &laid.first {
        None => writer.u8(0),
        Some(first) => {
            writer.u8(1);
            writer.u64(first.at);
            writer.u64(first.corrected_from);
            writer.len(first.metrics.len());
            for (metric, value) in &first.metrics {
                writer.u8(metric.code());
                writer.f64(*value);
            }
        }
    }
    writer.u32(laid.compares);
    match laid.result {
        None => writer.u8(0),
        Some((outcome, judge)) => {
            writer.u8(outcome.code());
            writer.u8(match judge {
                Judge::App => 1,
                Judge::Eye => 2,
            });
        }
    }
    writer.u32(laid.seconds);
}

/// The sheet stored under `key` as `bytes`.
pub(crate) fn decode_sheet(key: &[u8], bytes: &[u8]) -> Result<Sheet, PortError> {
    let mut reader = Reader::new(bytes, "practice sheet");
    let id = key
        .strip_prefix(&[PRACTICE.byte()])
        .and_then(|id| id.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or(reader.corrupt())?;
    if reader.u8()? != SHEET {
        return Err(reader.corrupt());
    }
    let (canvas, started) = (reader.u64()?, reader.u64()?);
    let purpose = match reader.u8()? {
        1 => Purpose::Day,
        2 => Purpose::Copy,
        3 => Purpose::Test,
        _ => return Err(reader.corrupt()),
    };
    let spans = read_spans(&mut reader)?;
    let count = reader.count(LESSON_BYTES)?;
    if count == 0 {
        return Err(reader.corrupt());
    }
    let mut lessons = Vec::with_capacity(count);
    for _ in 0..count {
        lessons.push(read_lesson(&mut reader)?);
    }
    reader.end()?;
    Ok(Sheet {
        id,
        canvas,
        started,
        purpose,
        spans,
        lessons,
    })
}

/// The spans the sheet was practised in; one ending before it starts is refused.
fn read_spans(reader: &mut Reader<'_>) -> Result<Vec<Span>, PortError> {
    let count = reader.count(SPAN_BYTES)?;
    let mut spans = Vec::with_capacity(count);
    for _ in 0..count {
        let span = Span {
            from: reader.u64()?,
            to: reader.u64()?,
        };
        if span.to < span.from {
            return Err(reader.corrupt());
        }
        spans.push(span);
    }
    Ok(spans)
}

fn read_lesson(reader: &mut Reader<'_>) -> Result<Lesson, PortError> {
    let (started, finished) = (reader.u64()?, reader.u64()?);
    let picked = match reader.u8()? {
        2 => Picked::Chosen,
        4 => Picked::First,
        5 => Picked::Retest {
            unit: reader.u8()?,
            end: reader.bool()?,
        },
        _ => return Err(reader.corrupt()),
    };
    let at = reader.u16()?;
    let count = reader.count(LAID_BYTES)?;
    let mut exercises = Vec::with_capacity(count);
    for _ in 0..count {
        exercises.push(read_laid(reader)?);
    }
    Ok(Lesson {
        started,
        finished: (finished != 0).then_some(finished),
        picked,
        at,
        exercises,
    })
}

fn read_laid(reader: &mut Reader<'_>) -> Result<Laid, PortError> {
    let (kind, variant, seed) = (reader.u8()?, reader.u8()?, reader.u64()?);
    let kind = Kind::of_code(kind, variant).ok_or(reader.corrupt())?;
    let level = reader.u8()?;
    let picture = match reader.u8()? {
        0 => None,
        1 => Some(ItemRef {
            pack: reader.str()?,
            item: reader.str()?,
        }),
        _ => return Err(reader.corrupt()),
    };
    let slot = reader.u16()?;
    let frame = [reader.u16()?, reader.u16()?];
    let first = match reader.u8()? {
        0 => None,
        1 => Some(read_first(reader)?),
        _ => return Err(reader.corrupt()),
    };
    let compares = reader.u32()?;
    let result = match reader.u8()? {
        0 => None,
        code => {
            let outcome = Outcome::of_code(code).ok_or(reader.corrupt())?;
            let judge = match reader.u8()? {
                1 => Judge::App,
                2 => Judge::Eye,
                _ => return Err(reader.corrupt()),
            };
            Some((outcome, judge))
        }
    };
    Ok(Laid {
        exercise: Exercise {
            picture,
            ..Exercise::generated(kind, seed)
        },
        level,
        slot,
        frame,
        first,
        compares,
        result,
        seconds: reader.u32()?,
    })
}

fn read_first(reader: &mut Reader<'_>) -> Result<First, PortError> {
    let (at, corrected_from) = (reader.u64()?, reader.u64()?);
    let count = reader.count(METRIC_BYTES)?;
    let mut metrics = Vec::with_capacity(count);
    for _ in 0..count {
        let (code, value) = (reader.u8()?, reader.f64()?);
        let metric = Metric::of_code(code).ok_or(reader.corrupt())?;
        metrics.push((metric, value));
    }
    Ok(First {
        at,
        corrected_from,
        metrics,
    })
}

/// The project the sheet record `bytes` names, read from its tag and the field after it alone,
/// whatever follows; none for a record of another tag or too short, and none for 0.
pub(crate) fn named_project(bytes: &[u8]) -> Option<u64> {
    let (&tag, rest) = bytes.split_first()?;
    let (project, _) = rest.split_first_chunk::<8>()?;
    let project = u64::from_le_bytes(*project);
    (tag == SHEET && project != 0).then_some(project)
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
