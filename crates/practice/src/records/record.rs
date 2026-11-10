//! What the sheets kept tell (`sheet.rs`), every lesson of each counted: the kinds met and the
//! pictures shown (what the next session or drill draws on), the value scales met, and a lesson's
//! summary (its means and colour judgement beside the earlier lessons of its kind). Long colour
//! copies count in none.

use crate::kinds::colours::judgement::Judgement;
use crate::packs::pack::ItemRef;
use crate::records::lessons::lessons;
use crate::records::metric::{Metric, Metrics, value};
use crate::records::sheet::{Lesson, Sheet};
use crate::sessions::plan::{Family, Kind};

/// The kinds compared in `sheets`' sessions and drills (long colour copies left out), each once,
/// in the order of their codes: what a warm-up draws from, and what the practice options draw in
/// ink.
#[must_use]
pub fn met(sheets: &[Sheet]) -> Vec<Kind> {
    let mut kinds: Vec<Kind> = compared(sheets).collect();
    kinds.sort_by_key(|kind| kind.code());
    kinds.dedup();
    kinds
}

/// How many value scales `sheets`' sessions and drills compared (S97: a beginner's first two are
/// short).
#[must_use]
pub fn scales_met(sheets: &[Sheet]) -> usize {
    let scale = |kind: &Kind| kind.family() == Family::Scale;
    compared(sheets).filter(scale).count()
}

/// The kind of every exercise `sheets`' sessions and drills compared (long colour copies left
/// out).
fn compared(sheets: &[Sheet]) -> impl Iterator<Item = Kind> + '_ {
    (sheets.iter().filter(|sheet| !sheet.is_copy())).flat_map(|sheet| {
        (sheet.exercises())
            .filter(|laid| laid.first.is_some())
            .map(|laid| laid.exercise.kind)
    })
}

/// The pictures `sheets`' sessions and drills showed (long colour copies left out), each once:
/// what the next session or drill picks others than.
#[must_use]
pub fn used(sheets: &[Sheet]) -> Vec<ItemRef> {
    let mut items: Vec<ItemRef> = (sheets.iter())
        .filter(|sheet| !sheet.is_copy())
        .flat_map(Sheet::exercises)
        .filter_map(|laid| laid.exercise.picture.clone())
        .collect();
    items.sort();
    items.dedup();
    items
}

/// The sessions and drills a composition waits before it comes back (S95).
pub const RECENT: usize = 10;

/// The pictures the last `RECENT` session and drill lessons of `sheets` (colour copies left out)
/// showed: what a division or a whole copy picks others than while there are.
#[must_use]
pub fn recent(sheets: &[Sheet]) -> Vec<ItemRef> {
    let mut items: Vec<ItemRef> = (lessons(sheets).iter().rev())
        .filter(|lesson| !lesson.is_copy())
        .take(RECENT)
        .flat_map(|lesson| lesson.lesson.exercises.iter())
        .filter_map(|laid| laid.exercise.picture.clone())
        .collect();
    items.sort();
    items.dedup();
    items
}

/// The metrics the summary sets side by side for colours, colour combinations and value scales.
pub const COLOUR_ROWS: [Metric; 3] = [Metric::FirstDelta, Metric::FinalDelta, Metric::Corrections];
pub const AREAS_ROWS: [Metric; 2] = [Metric::AreasDelta, Metric::AreasMadeStrong];
pub const SCALE_ROWS: [Metric; 2] = [Metric::ScaleOff, Metric::ScaleReversed];

/// One family of exercises in a session's summary.
#[derive(Debug, Clone, PartialEq)]
pub struct KindSummary {
    pub family: Family,
    /// Exercises of the kind compared, and held.
    pub checked: usize,
    pub held: usize,
    /// Colours, colour combinations and value scales: per metric the mean over this session's
    /// results and over the earlier sessions'; lines and shapes have none.
    pub rows: Vec<(Metric, Option<f64>, Option<f64>)>,
    /// Colours: this session's judgement and the earlier sessions'.
    pub judgement: Option<[Judgement; 2]>,
}

/// `this` lesson's summary per family it holds (the warm-up's exercises counted in their
/// families) beside `before` (the earlier lessons of its kind), in the order the session's mix
/// meets them (a family only warmed up with last).
#[must_use]
pub fn summary(this: &Lesson, before: &[&Lesson]) -> Vec<KindSummary> {
    let exercises: Vec<Kind> = (this.exercises.iter())
        .map(|laid| laid.exercise.kind)
        .collect();
    let warm_up = this.warm_up().min(exercises.len());
    let mut families = Family::ALL.to_vec();
    families.sort_by_key(|family| {
        (exercises[warm_up..].iter())
            .position(|kind| kind.family() == *family)
            .unwrap_or(usize::MAX)
    });
    families
        .into_iter()
        .filter_map(|family| {
            let held = (exercises.iter())
                .filter(|kind| kind.family() == family)
                .count();
            (held > 0).then(|| {
                let mine = results_of(&[this], family);
                let earlier = results_of(before, family);
                let rows = (family.facts().rows.iter())
                    .map(|m| (*m, mean(&mine, *m), mean(&earlier, *m)))
                    .collect();
                // The judgement its kind's description makes (a colour's radar, D108).
                let described = family.described();
                let judged = (described.judgement(&mine))
                    .zip(described.judgement(&earlier))
                    .map(|(this, before)| [this, before]);
                KindSummary {
                    family,
                    checked: mine.len(),
                    held,
                    rows,
                    judgement: judged,
                }
            })
        })
        .collect()
}

/// Every first Compare of `lessons` of `family`, with its exercise's kind.
fn results_of<'a>(lessons: &[&'a Lesson], family: Family) -> Vec<(Kind, &'a Metrics)> {
    (lessons.iter())
        .flat_map(|lesson| lesson.exercises.iter())
        .filter(|laid| laid.exercise.kind.family() == family)
        .filter_map(|laid| Some((laid.exercise.kind, &laid.first.as_ref()?.metrics)))
        .collect()
}

/// The mean of `metric` over the results holding it.
fn mean(results: &[(Kind, &Metrics)], metric: Metric) -> Option<f64> {
    let values: Vec<f64> = results
        .iter()
        .filter_map(|(_, m)| value(m, metric))
        .collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}
