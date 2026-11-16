//! One skill's ability over time, fitted as Whole-History Rating fits a player (Coulom 2008, a
//! dynamic Bradley-Terry model): an ability on each local day the skill was practised, wandering
//! between them as a Wiener process, and the exact maximum a posteriori over the whole history by
//! Newton's method, refitted from the stored observations whenever it is asked. Its uncertainty is
//! the Laplace approximation, the inverse Hessian at the maximum (Coulom's B.2).
//!
//! The unknowns: `θ_i` on each day (logits, relative), and how much harder the second and third
//! levels are, `Δ₂` and `Δ₃` (a level's difficulty `δ`: 0, `Δ₂`, `Δ₂ + Δ₃`). The prior: `θ_1 ~
//! N(0, 2²)` (where Coulom anchors the first day with virtual draws against a rating of 0), each
//! step `θ_{i+1} − θ_i ~ N(0, q² · min(gap, 60))` with `q² = 0.02` a day (Coulom's `w²`, as Glicko
//! lets a rating's deviation grow with time away), `Δ₂, Δ₃ ~ N(1.5, 0.75²)`. What is observed
//! (`terms.rs`): a result against its level's difficulty, a Rasch item being a Bradley-Terry
//! opponent; a kept measure in a result's place; a blind pair of two copies, a comparison between
//! the skill's abilities on their two days.
//!
//! Every term is log-concave, so the maximum is unique; Newton's step from `θ = 0, Δ = 1.5`, halved
//! until the objective falls, until no unknown moves by `STILL`. A walk is tridiagonal; a pair
//! links its two days, so the days' Hessian is a band (`band.rs`), the levels its border.
//!
//! One addition to Coulom's: the posterior spread is widened by how far the judgements scatter
//! about the fit, the mean squared Pearson residual and at least 1 (the quasi-likelihood dispersion,
//! Wedderburn 1974), so an eye whose answers disagree more than the model expects reads as less
//! sure; one as consistent as the model leaves the Laplace band as it is.

use crate::kinds::SkillLevel;
use crate::marks::compared::Outcome;
use crate::progress::band::{Factored, System};
use crate::progress::terms::{self, Term};

/// The walk's variance a day, the longest gap it grows over, the first day's spread.
const DRIFT: f64 = 0.02;
const LONGEST: i64 = 60;
const FIRST_SPREAD: f64 = 2.0;
/// The levels' prior: how much harder each is, and how sure.
const LEVEL_STEP: f64 = 1.5;
const LEVEL_SPREAD: f64 = 0.75;
/// Newton stops when no unknown moves by this; it gives up after `STEPS`, a step after `HALVINGS`.
const STILL: f64 = 1e-7;
const STEPS: usize = 100;
const HALVINGS: usize = 40;

/// What a blind pair's answer said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Better {
    Older,
    Alike,
    Newer,
}

/// One thing observed of a skill, on its days by their index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Observed {
    /// An exercise's result at a level.
    Result {
        day: usize,
        level: SkillLevel,
        outcome: Outcome,
    },
    /// A kept measure in a result's place (`z` on the result's scale).
    Measure {
        day: usize,
        level: SkillLevel,
        z: f64,
    },
    /// A blind pair of two copies, the answer and its weight `κ`.
    Pair {
        older: usize,
        newer: usize,
        better: Better,
        weight: f64,
    },
}

/// A skill's history: its practice days (local days, ascending, distinct) and what was observed.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct History {
    pub(crate) days: Vec<i64>,
    pub(crate) observed: Vec<Observed>,
}

/// A history fitted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Fit {
    /// Each day's ability (logits).
    pub(crate) ability: Vec<f64>,
    /// How much harder the second level is than the first, and the third than the second.
    pub(crate) levels: [f64; 2],
    /// The Newton steps it took.
    pub(crate) steps: usize,
    /// The judgements' dispersion about it, at least 1.
    pub(crate) dispersion: f64,
    factored: Option<Factored>,
}

impl Fit {
    /// `history` fitted; none without a day.
    pub(crate) fn of(history: &History) -> Option<Self> {
        let n = history.days.len();
        if n == 0 {
            return None;
        }
        let mut x = Unknowns {
            theta: vec![0.0; n],
            levels: [LEVEL_STEP; 2],
        };
        let mut steps = 0;
        while steps < STEPS {
            steps += 1;
            let (before, system, g, h) = summed(history, &x);
            let Some(factored) = system.factor() else {
                break;
            };
            let (dx, dy) = factored.solve(&g, h);
            let mut scale = 1.0;
            let mut moved = false;
            for _ in 0..HALVINGS {
                let tried = x.stepped(&dx, dy, scale);
                if summed(history, &tried).0 <= before + 1e-12 {
                    x = tried;
                    moved = true;
                    break;
                }
                scale /= 2.0;
            }
            let largest = dx.iter().chain(&dy).fold(0.0_f64, |m, d| m.max(d.abs()));
            if !moved || largest * scale < STILL {
                break;
            }
        }
        let (_, system, _, _) = summed(history, &x);
        Some(Self {
            dispersion: dispersion(history, &x),
            ability: x.theta,
            levels: x.levels,
            steps,
            factored: system.factor(),
        })
    }

    /// One standard deviation of day `b`'s ability less day `a`'s.
    pub(crate) fn spread(&self, a: usize, b: usize) -> f64 {
        if a == b {
            return 0.0;
        }
        let variance = (self.factored.as_ref()).map_or(0.0, |f| f.variance(&[(b, 1.0), (a, -1.0)]));
        (variance.max(0.0) * self.dispersion).sqrt()
    }

    /// For every day, one standard deviation of its ability less the first day's: the band.
    pub(crate) fn band(&self) -> Vec<f64> {
        (self.factored.as_ref())
            .map_or_else(
                || vec![0.0; self.ability.len()],
                Factored::spread_from_first,
            )
            .into_iter()
            .map(|variance| (variance * self.dispersion).sqrt())
            .collect()
    }
}

/// The unknowns: each day's ability and the two level steps.
#[derive(Debug, Clone)]
struct Unknowns {
    theta: Vec<f64>,
    levels: [f64; 2],
}

impl Unknowns {
    fn stepped(&self, dx: &[f64], dy: [f64; 2], scale: f64) -> Self {
        Self {
            theta: (self.theta.iter().zip(dx))
                .map(|(t, d)| t - scale * d)
                .collect(),
            levels: [
                self.levels[0] - scale * dy[0],
                self.levels[1] - scale * dy[1],
            ],
        }
    }

    /// Level `level`'s difficulty, and which level steps it is made of.
    fn difficulty(&self, level: SkillLevel) -> (f64, [f64; 2]) {
        match level {
            SkillLevel::First => (0.0, [0.0, 0.0]),
            SkillLevel::Second => (self.levels[0], [1.0, 0.0]),
            SkillLevel::Third => (self.levels[0] + self.levels[1], [1.0, 1.0]),
        }
    }

    /// `u = θ_day − δ` of an observation at `day` and `level`, and its level steps.
    fn against(&self, day: usize, level: SkillLevel) -> (f64, [f64; 2]) {
        let (delta, steps) = self.difficulty(level);
        (self.theta[day] - delta, steps)
    }
}

/// The negative log posterior at `x`, its Hessian as a system, and its gradient (days, levels).
fn summed(history: &History, x: &Unknowns) -> (f64, System, Vec<f64>, [f64; 2]) {
    let n = x.theta.len();
    let width = (history.observed.iter())
        .map(|seen| match seen {
            Observed::Pair { older, newer, .. } => older.abs_diff(*newer),
            _ => 1,
        })
        .fold(1, usize::max);
    let mut sum = Sum {
        value: 0.0,
        system: System::new(n, width),
        g: vec![0.0; n],
        h: [0.0; 2],
    };
    sum.prior(history, x);
    for seen in &history.observed {
        match *seen {
            Observed::Result {
                day,
                level,
                outcome,
            } => {
                let (u, steps) = x.against(day, level);
                sum.on_one(day, steps, terms::result(u, outcome));
            }
            Observed::Measure { day, level, z } => {
                let (u, steps) = x.against(day, level);
                sum.on_one(day, steps, terms::measure(u, z));
            }
            Observed::Pair {
                older,
                newer,
                better,
                weight,
            } => {
                let v = weight * (x.theta[newer] - x.theta[older]);
                sum.on_two([(newer, weight), (older, -weight)], terms::pair(v, better));
            }
        }
    }
    (sum.value, sum.system, sum.g, sum.h)
}

/// The mean squared Pearson residual of every observation at `x`, at least 1.
fn dispersion(history: &History, x: &Unknowns) -> f64 {
    let residuals: Vec<f64> = (history.observed.iter())
        .map(|seen| match *seen {
            Observed::Result {
                day,
                level,
                outcome,
            } => terms::result_residual(x.against(day, level).0, outcome),
            Observed::Measure { day, level, z } => {
                terms::measure_residual(x.against(day, level).0, z)
            }
            Observed::Pair {
                older,
                newer,
                better,
                weight,
            } => terms::pair_residual(weight * (x.theta[newer] - x.theta[older]), better),
        })
        .collect();
    if residuals.is_empty() {
        return 1.0;
    }
    (residuals.iter().sum::<f64>() / residuals.len() as f64).max(1.0)
}

/// The terms summed so far.
struct Sum {
    value: f64,
    system: System,
    g: Vec<f64>,
    h: [f64; 2],
}

impl Sum {
    /// The anchor on the first day, the walk between days and the levels' prior.
    fn prior(&mut self, history: &History, x: &Unknowns) {
        let first = FIRST_SPREAD * FIRST_SPREAD;
        self.value += x.theta[0].powi(2) / (2.0 * first);
        self.g[0] += x.theta[0] / first;
        self.system.band.add(0, 0, 1.0 / first);
        for i in 1..x.theta.len() {
            let gap = (history.days[i] - history.days[i - 1]).clamp(1, LONGEST);
            let v = DRIFT * gap as f64;
            let step = x.theta[i] - x.theta[i - 1];
            self.value += step * step / (2.0 * v);
            self.g[i] += step / v;
            self.g[i - 1] -= step / v;
            self.system.band.add(i, i, 1.0 / v);
            self.system.band.add(i - 1, i - 1, 1.0 / v);
            self.system.band.add(i, i - 1, -1.0 / v);
        }
        let spread = LEVEL_SPREAD * LEVEL_SPREAD;
        for k in 0..2 {
            let off = x.levels[k] - LEVEL_STEP;
            self.value += off * off / (2.0 * spread);
            self.h[k] += off / spread;
            self.system.corner[k][k] += 1.0 / spread;
        }
    }

    /// A term `(f, f', f'')` of `u = θ_day − δ`, `δ` made of the level steps `steps`.
    fn on_one(&mut self, day: usize, steps: [f64; 2], (f, d1, d2): Term) {
        self.value += f;
        self.g[day] += d1;
        self.system.band.add(day, day, d2);
        for k in 0..2 {
            self.h[k] -= d1 * steps[k];
            self.system.border[day][k] -= d2 * steps[k];
            for l in 0..2 {
                self.system.corner[k][l] += d2 * steps[k] * steps[l];
            }
        }
    }

    /// A term `(f, f', f'')` of `v = Σ a_i θ_i` over the two days `days`.
    fn on_two(&mut self, days: [(usize, f64); 2], (f, d1, d2): Term) {
        self.value += f;
        for (i, a) in days {
            self.g[i] += d1 * a;
            for (j, b) in days {
                if i >= j {
                    self.system.band.add(i, j, d2 * a * b);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The posterior's negative log, its maximum found by a crude search over one day's ability.
    #[test]
    fn with_one_day_and_one_level_the_fit_is_the_rating_scale_s_maximum_a_posteriori() {
        let outcomes = [Outcome::Clean, Outcome::Near, Outcome::Clean, Outcome::Redo];
        let observed = (outcomes.iter())
            .map(|outcome| Observed::Result {
                day: 0,
                level: SkillLevel::First,
                outcome: *outcome,
            })
            .collect();
        let history = History {
            days: vec![100],
            observed,
        };
        let fit = Fit::of(&history).expect("a day");
        let posterior = |t: f64| {
            let x = Unknowns {
                theta: vec![t],
                levels: [LEVEL_STEP; 2],
            };
            summed(&history, &x).0
        };
        let best = (0..=40_000)
            .map(|i| -4.0 + f64::from(i) * 2e-4)
            .min_by(|a, b| posterior(*a).total_cmp(&posterior(*b)))
            .expect("a grid");
        assert!(
            (fit.ability[0] - best).abs() < 3e-4,
            "{} {best}",
            fit.ability[0]
        );
        assert!(fit.steps < 12, "{} steps", fit.steps);
    }

    #[test]
    fn a_pair_answered_alike_between_equal_days_moves_nothing() {
        let one = |day| Observed::Result {
            day,
            level: SkillLevel::First,
            outcome: Outcome::Near,
        };
        let plain = History {
            days: vec![10, 40],
            observed: vec![one(0), one(1)],
        };
        let mut paired = plain.clone();
        paired.observed.push(Observed::Pair {
            older: 0,
            newer: 1,
            better: Better::Alike,
            weight: 1.0,
        });
        let (a, b) = (
            Fit::of(&plain).expect("fit"),
            Fit::of(&paired).expect("fit"),
        );
        for (x, y) in a.ability.iter().zip(&b.ability) {
            assert!((x - y).abs() < 1e-9);
        }
        assert!(b.spread(0, 1) < a.spread(0, 1), "the pair still tells");
    }
}
