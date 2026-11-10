//! The one-time conversion of the session records (S120c): a record of 1.7.0 to 1.21.6
//! (`old.rs`) becomes a sheet record of one lesson (`records.rs`) and that lesson's givens, all
//! of them in one write when the runtime's storage worker opens the library; a record already a
//! sheet's is left as it is, so a second run writes nothing. Deleted by S121, once a backup shows
//! every practice record in the new shape.
//!
//! A long colour copy (stage 0) becomes a copy's sheet, a drill (stage 255) a day's sheet of a
//! kind chosen, a session a day's sheet of its stage's lesson with its warm-up. Exercise i lies in
//! slot i at level 0 with no seconds of its own; its frames are its picture's as the packs hold
//! it (square for none), a copy's read from its canvas (`Layout::study_on`); its first Compare
//! and compares are the old result's, its outcome the app's where the numbers kept settle it
//! (`openpaint_practice::settled`). Its givens are what today's generators print.

mod old;

use openpaint_app::{PortError, ProjectId, ProjectStore, Record, Space};
use openpaint_practice::{
    First, Givens, Judge, Kind, Laid, Layout, Lesson, Pack, Picked, Purpose, Sheet, Span, settled,
};

use crate::givens::encode_givens;
use crate::records::{GIVENS, PRACTICE, SHEET, encode_sheet, givens_key};
use old::{Old, decode_old};

/// What an old record held, as the conversion's summary and its proof read it.
#[derive(Debug, Clone, PartialEq)]
pub struct Was {
    pub id: u64,
    /// Its stage (0 a long colour copy, 255 a drill) and number.
    pub stage: u8,
    pub number: u32,
    pub canvas: u64,
    pub finished: bool,
    pub spans: Vec<Span>,
    /// Each exercise compared, by its index, and its kind.
    pub compared: Vec<(u32, Kind)>,
    pub exercises: usize,
}

/// What a conversion did: each old record with the sheet it became, and how many old records
/// could not be read (left as they were, logged).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Converted {
    pub sheets: Vec<(Was, Sheet)>,
    pub unread: usize,
}

/// Every old session record in `store` converted to a sheet record and its givens (pictures as
/// `packs` hold them), all in one write; nothing written when none is old. Each conversion is
/// logged, one line a record and the totals.
///
/// # Errors
/// Whatever reading or writing the library's records fails with; nothing is written then.
pub fn convert(store: &mut dyn ProjectStore, packs: &[Pack]) -> Result<Converted, PortError> {
    let mut converted = Converted::default();
    let mut writes: Vec<(Space, Record)> = Vec::new();
    for (key, value) in store.records(PRACTICE, &[])? {
        if value.first() == Some(&SHEET) {
            continue;
        }
        let old = match decode_old(&key, &value) {
            Ok(old) => old,
            Err(error) => {
                log::error!("a practice record is not converted: {error}");
                converted.unread += 1;
                continue;
            }
        };
        let sheet = sheet_of(store, &old, packs);
        writes.push((PRACTICE, (key, encode_sheet(&sheet))));
        if let Some(lesson) = sheet.lessons.first().filter(|_| sheet.canvas != 0) {
            let givens = Givens::of_lesson(lesson, packs);
            writes.push((
                GIVENS,
                (givens_key(sheet.canvas, 0), encode_givens(&givens)),
            ));
        }
        converted.sheets.push((was(&old), sheet));
    }
    if writes.is_empty() {
        return Ok(converted);
    }
    store.put_records(&writes)?;
    for line in converted.summary() {
        log::info!("practice record converted: {line}");
    }
    Ok(converted)
}

/// The sheet `old` becomes, a copy's frames read from its canvas in `store`.
fn sheet_of(store: &mut dyn ProjectStore, old: &Old, packs: &[Pack]) -> Sheet {
    let copy = old.stage == 0;
    let picked = match old.stage {
        0 | u8::MAX => Picked::Chosen,
        stage => Picked::Full {
            stage,
            warm_up: u8::try_from(old.warm_up).unwrap_or(u8::MAX),
        },
    };
    let exercises = (old.exercises.iter().enumerate())
        .map(|(index, exercise)| {
            let mut laid = Laid::new(exercise.clone(), index);
            let result = old.results.iter().find(|r| r.index as usize == index);
            if let Some(result) = result {
                laid.first = Some(First {
                    at: result.checked_at,
                    corrected_from: result.corrected_from,
                    metrics: result.metrics.clone(),
                });
                laid.compares = result.compares;
                laid.result = settled(exercise, &result.metrics).map(|o| (o, Judge::App));
            }
            laid
        })
        .collect();
    let sheet = Sheet {
        id: old.id,
        canvas: old.canvas,
        started: old.started,
        purpose: if copy { Purpose::Copy } else { Purpose::Day },
        spans: old.time.clone(),
        lessons: vec![Lesson {
            started: old.started,
            finished: old.finished,
            picked,
            at: u16::try_from(old.current).unwrap_or(u16::MAX),
            exercises,
        }],
    };
    let mut sheet = sheet.framed(packs);
    if copy && old.canvas != 0 {
        match store.artwork(ProjectId::new(old.canvas)) {
            Ok(artwork) => {
                let canvas = artwork.canvas();
                let laid = Layout::study_on([canvas.width(), canvas.height()]);
                let size = laid.pairs[0].copy.size.map(|side| side.round() as u16);
                if let Some(first) = sheet.laid_mut(0) {
                    first.frame = size;
                }
            }
            Err(error) => log::warn!(
                "colour copy {}'s canvas not read, its frames its picture's: {error}",
                old.id
            ),
        }
    }
    sheet
}

fn was(old: &Old) -> Was {
    let compared = (old.results.iter())
        .filter_map(|r| Some((r.index, old.exercises.get(r.index as usize)?.kind)))
        .collect();
    Was {
        id: old.id,
        stage: old.stage,
        number: old.number,
        canvas: old.canvas,
        finished: old.finished.is_some(),
        spans: old.time.clone(),
        compared,
        exercises: old.exercises.len(),
    }
}

impl Converted {
    /// One line a record converted (`id purpose picked stage/number exercises compared` and each
    /// outcome kept, `index:outcome`), then the totals.
    #[must_use]
    pub fn summary(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.sheets.len() + 1);
        let mut outcomes = [0_usize; 3];
        for (was, sheet) in &self.sheets {
            let picked = match sheet.lesson().map(|lesson| lesson.picked) {
                Some(Picked::Full { stage, warm_up }) => format!("full({stage},{warm_up})"),
                _ => "chosen".to_owned(),
            };
            let kept: Vec<String> = (sheet.exercises().enumerate())
                .filter_map(|(index, laid)| Some((index, laid.result?.0)))
                .map(|(index, outcome)| {
                    outcomes[usize::from(outcome.code() - 1)] += 1;
                    format!("{index}:{outcome:?}")
                })
                .collect();
            lines.push(format!(
                "{} {:?} {picked} {}/{} {} {} {}",
                was.id,
                sheet.purpose,
                was.stage,
                was.number,
                was.exercises,
                was.compared.len(),
                kept.join(" ")
            ));
        }
        let compared: usize = self.sheets.iter().map(|(was, _)| was.compared.len()).sum();
        let [clean, near, redo] = outcomes;
        lines.push(format!(
            "{} records converted, {} unread: {compared} exercises compared, outcomes {clean} \
             clean {near} near {redo} redo",
            self.sheets.len(),
            self.unread,
        ));
        lines
    }
}
