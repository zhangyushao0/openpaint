//! What is kept of a session: its stage and place in it, its seed and exercises, where its sheet
//! is, its progress and every checked exercise's numbers with when it was checked. Enough for the
//! summary now and for trajectories, re-tests and drills later. The summary sets this session's
//! means beside those of the sessions before it in its stage.

use crate::metric::{Metric, Metrics, value};
use crate::plan::{Exercise, stage_one};

/// One session as it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRecord {
    /// Its number among all sessions, from 1; never reused.
    pub id: u64,
    /// Its stage, 1 to `STAGES`.
    pub stage: u8,
    /// Its number within its stage, from 1.
    pub number: u32,
    pub seed: u64,
    /// The project that holds its sheet (the app's project id); 0 until it has one.
    pub canvas: u64,
    /// When it started and finished, in seconds since 1970 UTC.
    pub started: u64,
    pub finished: Option<u64>,
    /// The exercise shown, an index into `exercises`.
    pub current: u32,
    pub exercises: Vec<Exercise>,
    /// The exercises checked, in the order they were.
    pub results: Vec<ExerciseResult>,
}

/// One checked exercise.
#[derive(Debug, Clone, PartialEq)]
pub struct ExerciseResult {
    /// Its index in the session's exercises.
    pub index: u32,
    /// When it was checked, in seconds since 1970 UTC.
    pub checked_at: u64,
    pub metrics: Metrics,
}

impl SessionRecord {
    /// Session `number` of stage 1, the `id`th of all, seeded `seed`, started at `started`.
    #[must_use]
    pub fn stage_one(id: u64, number: u32, seed: u64, started: u64) -> Self {
        Self {
            id,
            stage: 1,
            number,
            seed,
            canvas: 0,
            started,
            finished: None,
            current: 0,
            exercises: stage_one(seed),
            results: Vec::new(),
        }
    }

    /// The result of exercise `index`, once it was checked.
    #[must_use]
    pub fn result(&self, index: usize) -> Option<&ExerciseResult> {
        self.results.iter().find(|r| r.index as usize == index)
    }

    /// The exercise shown.
    #[must_use]
    pub fn exercise(&self) -> Option<&Exercise> {
        self.exercises.get(self.current as usize)
    }

    /// Of the kind of exercise `index` (lines or colours): its place among them, from 1, and how
    /// many the session holds.
    #[must_use]
    pub fn place(&self, index: usize) -> (usize, usize) {
        let Some(line) = self.exercises.get(index).map(|e| e.kind.is_line()) else {
            return (0, 0);
        };
        let alike = |e: &&Exercise| e.kind.is_line() == line;
        let before = self.exercises[..index].iter().filter(alike).count();
        (before + 1, self.exercises.iter().filter(alike).count())
    }
}

/// The metrics the summary sets side by side for lines and for colours.
pub const LINE_ROWS: [Metric; 5] = [
    Metric::KeyPoints,
    Metric::Frame,
    Metric::Shape,
    Metric::Speed,
    Metric::Wobble,
];
pub const COLOUR_ROWS: [Metric; 3] = [Metric::FirstDelta, Metric::FinalDelta, Metric::Corrections];

/// One kind of exercise in a session's summary.
#[derive(Debug, Clone, PartialEq)]
pub struct KindSummary {
    pub lines: bool,
    /// Exercises of the kind checked, and held.
    pub checked: usize,
    pub held: usize,
    /// Per metric: the mean over this session's results and over the earlier sessions'.
    pub rows: Vec<(Metric, Option<f64>, Option<f64>)>,
}

/// `this` session's means per kind beside those of `before` (the earlier sessions of its stage).
#[must_use]
pub fn summary(this: &SessionRecord, before: &[&SessionRecord]) -> Vec<KindSummary> {
    [true, false]
        .into_iter()
        .filter_map(|lines| {
            let held = this
                .exercises
                .iter()
                .filter(|e| e.kind.is_line() == lines)
                .count();
            (held > 0).then(|| {
                let mine = results_of(&[this], lines);
                let earlier = results_of(before, lines);
                let rows = if lines {
                    &LINE_ROWS[..]
                } else {
                    &COLOUR_ROWS[..]
                };
                KindSummary {
                    lines,
                    checked: mine.len(),
                    held,
                    rows: rows
                        .iter()
                        .map(|&m| (m, mean(&mine, m), mean(&earlier, m)))
                        .collect(),
                }
            })
        })
        .collect()
}

/// Every result of `sessions` of lines (or colours).
fn results_of<'a>(sessions: &[&'a SessionRecord], lines: bool) -> Vec<&'a Metrics> {
    sessions
        .iter()
        .flat_map(|session| {
            session.results.iter().filter(move |result| {
                session
                    .exercises
                    .get(result.index as usize)
                    .is_some_and(|e| e.kind.is_line() == lines)
            })
        })
        .map(|result| &result.metrics)
        .collect()
}

/// The mean of `metric` over the results holding it.
fn mean(results: &[&Metrics], metric: Metric) -> Option<f64> {
    let values: Vec<f64> = results.iter().filter_map(|m| value(m, metric)).collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}
