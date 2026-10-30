//! What is kept of a session: its stage and place in it, its seed and exercises (the first of
//! them its warm-up), where its sheet is, its progress and every compared exercise (when first
//! compared, how often, which strokes are corrections, a colour's, combination's or scale's
//! numbers) and the spans its sheet was practised in (`time.rs`, S89): enough for the summary
//! (this session's means and colour judgement beside its stage's earlier sessions'), the next
//! warm-up, the practice book, and later trajectories, re-tests and a study of the strokes.
//!
//! A long colour copy (V大's 色彩临摹) is kept the same way, as a project beside the sessions:
//! stage `PROJECT` (none of the course's), its number among the copies, one exercise, the copy of
//! its picture, never finished by a session's Next. It counts in no stage, warm-up or summary.
//!
//! A drill (S91, `drill.rs`) too: stage `DRILL` (past the course's eight), its number among its
//! kind's drills, exercises of one kind, which is the kind it drills. It counts in the kinds met,
//! the pictures shown, the time and the seals of lines and colours, never as a session.

use crate::colour_match::Tone;
use crate::judgement::{Judgement, judgement};
use crate::metric::{Metric, Metrics, value};
use crate::pack::ItemRef;
use crate::plan::{Exercise, Family, Kind};
use crate::session::{Plan, stage_one};
use crate::time::{Span, spans_of};

/// The stage a long colour copy's record holds: none of the course's.
pub const PROJECT: u8 = 0;
/// The stage a drill's record holds: none of the course's, past its eight.
pub const DRILL: u8 = u8::MAX;

/// One session as it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRecord {
    /// Its number among all sessions and copies, from 1; never reused.
    pub id: u64,
    /// Its stage, 1 to `STAGES`; `PROJECT` for a long colour copy.
    pub stage: u8,
    /// Its number within its stage (among the copies), from 1.
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
    /// The spans its sheet was practised in, oldest first (`time.rs`).
    pub time: Vec<Span>,
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
    /// The sheet's first stroke id after the first Compare: strokes begun in its frame from it on
    /// are corrections; `None` in 1.1.0's records (a compared frame took nothing more).
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
        Self::begun(id, (1, number), (seed, started), stage_one(seed, met))
    }

    /// Session `number` of `stage`, the `id`th of all, seeded `seed`, started at `started`,
    /// holding `plan` (made from the same seed).
    #[must_use]
    pub fn begun(
        id: u64,
        (stage, number): (u8, u32),
        (seed, started): (u64, u64),
        plan: Plan,
    ) -> Self {
        Self {
            id,
            stage,
            number,
            seed,
            canvas: 0,
            started,
            finished: None,
            current: 0,
            exercises: plan.exercises,
            warm_up: plan.warm_up,
            results: Vec::new(),
            time: Vec::new(),
        }
    }

    /// The spans a record kept before S89 practised, as the times it holds tell them: when it
    /// began, when each exercise was first compared and when it finished, joined where at most
    /// `PAUSE` apart (`time.rs`).
    #[must_use]
    pub fn time_of_compares(&self) -> Vec<Span> {
        let compared = self.results.iter().map(|result| result.checked_at);
        let times: Vec<u64> = (std::iter::once(self.started).chain(compared))
            .chain(self.finished)
            .collect();
        spans_of(&times)
    }

    /// Long colour copy `number`, the `id`th of all sessions and copies, seeded `seed`, begun at
    /// `started`, of `picture`.
    #[must_use]
    pub fn project(id: u64, number: u32, (seed, started): (u64, u64), picture: ItemRef) -> Self {
        let copy = Exercise {
            picture: Some(picture),
            ..Exercise::generated(Kind::Study, seed)
        };
        let plan = Plan {
            exercises: vec![copy],
            warm_up: 0,
        };
        Self::begun(id, (PROJECT, number), (seed, started), plan)
    }

    /// Drill `number` of its kind, the `id`th of all records, seeded `seed`, begun at `started`,
    /// holding `plan` (`drill.rs`, made from the same seed).
    #[must_use]
    pub fn drill(id: u64, number: u32, (seed, started): (u64, u64), plan: Plan) -> Self {
        Self::begun(id, (DRILL, number), (seed, started), plan)
    }

    /// Whether it is a long colour copy, not a session.
    #[must_use]
    pub const fn is_project(&self) -> bool {
        self.stage == PROJECT
    }

    /// Whether it is a drill, not a session.
    #[must_use]
    pub const fn is_drill(&self) -> bool {
        self.stage == DRILL
    }

    /// Whether it is a session of the course's stages: neither a colour copy nor a drill.
    #[must_use]
    pub const fn is_session(&self) -> bool {
        !self.is_project() && !self.is_drill()
    }

    /// The kind a drill drills (its exercises'); none for a session or a copy.
    #[must_use]
    pub fn drilled(&self) -> Option<Family> {
        let first = self.exercises.first().filter(|_| self.is_drill())?;
        Some(first.kind.family())
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

/// The kinds compared in `sessions` and drills (long colour copies left out), each once, in the
/// order of their codes: what a warm-up draws from, and what the practice options draw in ink.
#[must_use]
pub fn met(sessions: &[SessionRecord]) -> Vec<Kind> {
    let mut kinds: Vec<Kind> = compared(sessions).collect();
    kinds.sort_by_key(|kind| kind.code());
    kinds.dedup();
    kinds
}

/// How many value scales `sessions` and drills compared (S97: a beginner's first two are short).
#[must_use]
pub fn scales_met(sessions: &[SessionRecord]) -> usize {
    let scale = |kind: &Kind| kind.family() == Family::Scale;
    compared(sessions).filter(scale).count()
}

/// The kind of every exercise `sessions` and drills compared (long colour copies left out).
fn compared(sessions: &[SessionRecord]) -> impl Iterator<Item = Kind> + '_ {
    (sessions.iter().filter(|session| !session.is_project())).flat_map(|session| {
        (session.results.iter())
            .filter_map(|result| session.exercises.get(result.index as usize))
            .map(|exercise| exercise.kind)
    })
}

/// The pictures `sessions` and drills showed (long colour copies left out), each once: what the
/// next session or drill picks others than.
#[must_use]
pub fn used(sessions: &[SessionRecord]) -> Vec<ItemRef> {
    let mut items: Vec<ItemRef> = (sessions.iter())
        .filter(|session| !session.is_project())
        .flat_map(|session| session.exercises.iter())
        .filter_map(|exercise| exercise.picture.clone())
        .collect();
    items.sort();
    items.dedup();
    items
}

/// The sessions and drills a composition waits before it comes back (S95).
pub const RECENT: usize = 10;

/// The pictures the last `RECENT` sessions and drills of `sessions` (oldest first; colour copies
/// left out) showed: what a division or a whole copy picks others than while there are.
#[must_use]
pub fn recent(sessions: &[SessionRecord]) -> Vec<ItemRef> {
    let mut items: Vec<ItemRef> = (sessions.iter().rev())
        .filter(|session| !session.is_project())
        .take(RECENT)
        .flat_map(|session| session.exercises.iter())
        .filter_map(|exercise| exercise.picture.clone())
        .collect();
    items.sort();
    items.dedup();
    items
}

/// The metrics the summary sets side by side for colours, colour combinations and value scales.
pub const COLOUR_ROWS: [Metric; 3] = [Metric::FirstDelta, Metric::FinalDelta, Metric::Corrections];
pub const AREAS_ROWS: [Metric; 2] = [Metric::AreasDelta, Metric::AreasMadeStrong];
pub const SCALE_ROWS: [Metric; 2] = [Metric::ScaleOff, Metric::ScaleReversed];

/// The metrics the summary sets side by side for `family`.
const fn rows_of(family: Family) -> &'static [Metric] {
    match family {
        Family::Colour => &COLOUR_ROWS,
        Family::Areas => &AREAS_ROWS,
        Family::Scale => &SCALE_ROWS,
        // Estimates, points in space and solids keep their errors for the trajectories, never
        // shown (D107); a division its counts and lengths, shown on its card only (S95); value
        // masses their share at the answer's value, shown nowhere (S96).
        Family::Line
        | Family::Combined
        | Family::Shape
        | Family::Box
        | Family::FreeBoxes
        | Family::Ellipse
        | Family::Point
        | Family::Proportion
        | Family::Angle
        | Family::Centre
        | Family::Halves
        | Family::Thirds
        | Family::Step
        | Family::Mirror
        | Family::Cylinder
        | Family::Cone
        | Family::Sphere
        | Family::Object
        | Family::Division
        | Family::WholeCopy
        | Family::ValueMasses
        | Family::Study => &[],
    }
}

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

/// `this` session's summary per family it holds (the warm-up's exercises counted in their
/// families) beside `before` (the earlier sessions of its stage), in the order the session's mix
/// meets them (a family only warmed up with last).
#[must_use]
pub fn summary(this: &SessionRecord, before: &[&SessionRecord]) -> Vec<KindSummary> {
    let warm_up = (this.warm_up as usize).min(this.exercises.len());
    let mut families = Family::ALL.to_vec();
    families.sort_by_key(|family| {
        (this.exercises[warm_up..].iter())
            .position(|e| e.kind.family() == *family)
            .unwrap_or(usize::MAX)
    });
    families
        .into_iter()
        .filter_map(|family| {
            let held = (this.exercises.iter())
                .filter(|e| e.kind.family() == family)
                .count();
            (held > 0).then(|| {
                let mine = results_of(&[this], family);
                let earlier = results_of(before, family);
                let rows = (rows_of(family).iter())
                    .map(|m| (*m, mean(&mine, *m), mean(&earlier, *m)))
                    .collect();
                let judged = (family == Family::Colour).then(|| [judged(&mine), judged(&earlier)]);
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
            _ => None,
        })
        .collect();
    judgement(&colours)
}
