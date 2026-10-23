//! What is kept of a session: its stage and place in it, its seed and exercises (the first of
//! them its warm-up), where its sheet is, its progress and every compared exercise: when it was
//! first compared, how often, which of its strokes are corrections and, for a colour, its
//! numbers. Enough for the summary and the next warm-up now, and for trajectories, re-tests,
//! drills and a study of the strokes later. The summary sets this session's colour means and
//! judgement beside those of the sessions before it in its stage.

use crate::colour_match::Tone;
use crate::judgement::{Judgement, judgement};
use crate::metric::{Metric, Metrics, value};
use crate::plan::{Exercise, Family, Kind};
use crate::session::stage_one;

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
    /// How many of the first exercises are the warm-up.
    pub warm_up: u32,
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
    /// Session `number` of stage 1, the `id`th of all, seeded `seed`, started at `started`, after
    /// the kinds `met` in earlier sessions (its warm-up's, `met`).
    #[must_use]
    pub fn stage_one(id: u64, number: u32, seed: u64, started: u64, met: &[Kind]) -> Self {
        let plan = stage_one(seed, met);
        Self {
            id,
            stage: 1,
            number,
            seed,
            canvas: 0,
            started,
            finished: None,
            current: 0,
            exercises: plan.exercises,
            warm_up: plan.warm_up,
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

    /// Whether exercise `index` is part of the warm-up.
    #[must_use]
    pub fn warming_up(&self, index: usize) -> bool {
        index < self.warm_up as usize
    }

    /// Exercise `index`'s place, from 1, and how many there are: in the warm-up among the
    /// warm-up's exercises, after it among those of its family after it.
    #[must_use]
    pub fn place(&self, index: usize) -> (usize, usize) {
        let Some(family) = self.exercises.get(index).map(|e| e.kind.family()) else {
            return (0, 0);
        };
        let warm_up = (self.warm_up as usize).min(self.exercises.len());
        if index < warm_up {
            return (index + 1, warm_up);
        }
        let alike = |e: &&Exercise| e.kind.family() == family;
        let before = self.exercises[warm_up..index].iter().filter(alike).count();
        let all = self.exercises[warm_up..].iter().filter(alike).count();
        (before + 1, all)
    }
}

/// The kinds compared in `sessions`, each once, in the order of their codes: what a warm-up
/// draws from.
#[must_use]
pub fn met(sessions: &[SessionRecord]) -> Vec<Kind> {
    let mut kinds: Vec<Kind> = sessions
        .iter()
        .flat_map(|session| {
            (session.results.iter())
                .filter_map(|result| session.exercises.get(result.index as usize))
                .map(|exercise| exercise.kind)
        })
        .collect();
    kinds.sort_by_key(|kind| kind.code());
    kinds.dedup();
    kinds
}

/// The metrics the summary sets side by side for colours.
pub const COLOUR_ROWS: [Metric; 3] = [Metric::FirstDelta, Metric::FinalDelta, Metric::Corrections];

/// One family of exercises in a session's summary.
#[derive(Debug, Clone, PartialEq)]
pub struct KindSummary {
    pub family: Family,
    /// Exercises of the kind compared, and held.
    pub checked: usize,
    pub held: usize,
    /// Colours: per metric the mean over this session's results and over the earlier sessions';
    /// lines have none.
    pub rows: Vec<(Metric, Option<f64>, Option<f64>)>,
    /// Colours: this session's judgement and the earlier sessions'.
    pub judgement: Option<[Judgement; 2]>,
}

/// `this` session's summary per family it holds (the warm-up's exercises counted in their
/// families) beside `before` (the earlier sessions of its stage).
#[must_use]
pub fn summary(this: &SessionRecord, before: &[&SessionRecord]) -> Vec<KindSummary> {
    Family::ALL
        .into_iter()
        .filter_map(|family| {
            let held = (this.exercises.iter())
                .filter(|e| e.kind.family() == family)
                .count();
            (held > 0).then(|| {
                let mine = results_of(&[this], family);
                let earlier = results_of(before, family);
                let (rows, judged) = if family.drawn() {
                    (Vec::new(), None)
                } else {
                    let rows = COLOUR_ROWS.map(|m| (m, mean(&mine, m), mean(&earlier, m)));
                    (rows.to_vec(), Some([judged(&mine), judged(&earlier)]))
                };
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

/// Every result of `sessions` of `family`, with its exercise's kind.
fn results_of<'a>(sessions: &[&'a SessionRecord], family: Family) -> Vec<(Kind, &'a Metrics)> {
    sessions
        .iter()
        .flat_map(|session| {
            session.results.iter().filter_map(move |result| {
                let exercise = session.exercises.get(result.index as usize)?;
                (exercise.kind.family() == family).then_some((exercise.kind, &result.metrics))
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
            Kind::Line(_) | Kind::Combined(_) => None,
        })
        .collect();
    judgement(&colours)
}
