//! What is kept of a session: its stage and place in it, its seed and exercises, where its sheet
//! is, its progress and every compared exercise: when it was first compared, how often, which of
//! its strokes are corrections and, for a colour, its numbers. Enough for the summary now and for
//! trajectories, re-tests, drills and a study of the strokes later. The summary sets this
//! session's colour means and judgement beside those of the sessions before it in its stage.

use crate::colour_match::Tone;
use crate::judgement::{Judgement, judgement};
use crate::metric::{Metric, Metrics, value};
use crate::plan::{Exercise, Kind, stage_one};

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
    /// The exercises compared, in the order they first were.
    pub results: Vec<ExerciseResult>,
}

/// One compared exercise.
#[derive(Debug, Clone, PartialEq)]
pub struct ExerciseResult {
    /// Its index in the session's exercises.
    pub index: u32,
    /// When it was first compared, in seconds since 1970 UTC.
    pub checked_at: u64,
    /// How often it was compared, from 1.
    pub compares: u32,
    /// The sheet's first stroke id after the first Compare: the strokes begun in its frame with
    /// this id or a later one are corrections. `None` in 1.1.0's records, where a compared frame
    /// took nothing more.
    pub corrected_from: Option<u64>,
    /// A colour's numbers at its first Compare; none for a line.
    pub metrics: Metrics,
}

impl ExerciseResult {
    /// Exercise `index` compared for the first time at `checked_at`, the sheet's next stroke id
    /// then `next_stroke`, with `metrics`.
    #[must_use]
    pub const fn first(index: u32, checked_at: u64, next_stroke: u64, metrics: Metrics) -> Self {
        Self {
            index,
            checked_at,
            compares: 1,
            corrected_from: Some(next_stroke),
            metrics,
        }
    }

    /// Whether the stroke `id` begun in its frame is a correction.
    #[must_use]
    pub fn corrects(&self, id: u64) -> bool {
        self.corrected_from.is_some_and(|from| id >= from)
    }
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

/// The metrics the summary sets side by side for colours.
pub const COLOUR_ROWS: [Metric; 3] = [Metric::FirstDelta, Metric::FinalDelta, Metric::Corrections];

/// One kind of exercise in a session's summary.
#[derive(Debug, Clone, PartialEq)]
pub struct KindSummary {
    pub lines: bool,
    /// Exercises of the kind compared, and held.
    pub checked: usize,
    pub held: usize,
    /// Colours: per metric the mean over this session's results and over the earlier sessions';
    /// lines have none.
    pub rows: Vec<(Metric, Option<f64>, Option<f64>)>,
    /// Colours: this session's judgement and the earlier sessions'.
    pub judgement: Option<[Judgement; 2]>,
}

/// `this` session's summary per kind beside `before` (the earlier sessions of its stage).
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
                let (rows, judged) = if lines {
                    (Vec::new(), None)
                } else {
                    let rows = COLOUR_ROWS.map(|m| (m, mean(&mine, m), mean(&earlier, m)));
                    (rows.to_vec(), Some([judged(&mine), judged(&earlier)]))
                };
                KindSummary {
                    lines,
                    checked: mine.len(),
                    held,
                    rows,
                    judgement: judged,
                }
            })
        })
        .collect()
}

/// Every result of `sessions` of lines (or colours), with its exercise's kind.
fn results_of<'a>(sessions: &[&'a SessionRecord], lines: bool) -> Vec<(Kind, &'a Metrics)> {
    sessions
        .iter()
        .flat_map(|session| {
            session.results.iter().filter_map(move |result| {
                let exercise = session.exercises.get(result.index as usize)?;
                (exercise.kind.is_line() == lines).then_some((exercise.kind, &result.metrics))
            })
        })
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

/// The judgement of the colour results among `results`.
fn judged(results: &[(Kind, &Metrics)]) -> Judgement {
    let colours: Vec<(Tone, &[(Metric, f64)])> = results
        .iter()
        .filter_map(|(kind, metrics)| match kind {
            Kind::Colour(tone) => Some((*tone, metrics.as_slice())),
            Kind::Line(_) => None,
        })
        .collect();
    judgement(&colours)
}
