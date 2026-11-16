//! What one observation adds to the progress fit (`fit.rs`), as a function of the one combination
//! of unknowns it reads: its negative log-likelihood and the first two derivatives, and how far it
//! lies from what the fit expects (its Pearson residual, squared).
//!
//! - A result: Andrich's rating scale model at `u = θ − δ`, thresholds `(−1, +1)`: `P(x) ∝ exp
//!   Σ_{k ≤ x} (u − τ_k)` for 重来, 差一点, 准 (`x` 0, 1, 2); an exponential family in `u`, so its
//!   derivatives are `E[x] − x` and `Var[x]`.
//! - A measure: `z ~ N(u, 1.8²)`.
//! - A blind pair: `v = κ (θ_newer − θ_older)`, `P(newer) = σ(v − η)`, `P(older) = σ(−v − η)`,
//!   alike the rest (Rao-Kupper's threshold on a logistic, an ordered logit, log-concave).

use crate::marks::compared::Outcome;
use crate::progress::fit::Better;

/// The rating scale's thresholds, a measure's spread and a pair's tie threshold.
const THRESHOLDS: [f64; 2] = [-1.0, 1.0];
pub(crate) const MEASURE_SPREAD: f64 = 1.8;
const TIE: f64 = 0.85;

/// A term's value and first two derivatives.
pub(crate) type Term = (f64, f64, f64);

/// The rating scale's sums `Σ_{k ≤ x} (u − τ_k)` at `u`, for `x` 0, 1, 2.
fn sums(u: f64) -> [f64; 3] {
    [
        0.0,
        u - THRESHOLDS[0],
        2.0 * u - THRESHOLDS[0] - THRESHOLDS[1],
    ]
}

/// The rating scale's three shares at `u`, and the log of their normaliser.
fn shares(u: f64) -> ([f64; 3], f64) {
    let s = sums(u);
    let top = s.iter().copied().fold(f64::MIN, f64::max);
    let weights = s.map(|s| (s - top).exp());
    let total: f64 = weights.iter().sum();
    (weights.map(|w| w / total), top + total.ln())
}

const fn score(outcome: Outcome) -> u8 {
    match outcome {
        Outcome::Redo => 0,
        Outcome::Near => 1,
        Outcome::Clean => 2,
    }
}

/// A result's `−log P(outcome)` at `u`, in `u`.
pub(crate) fn result(u: f64, outcome: Outcome) -> Term {
    let x = score(outcome);
    let (p, log_total) = shares(u);
    let s = sums(u);
    let mean = p[1] + 2.0 * p[2];
    let square = p[1] + 4.0 * p[2];
    (
        log_total - s[usize::from(x)],
        mean - f64::from(x),
        square - mean * mean,
    )
}

/// A result's squared Pearson residual at `u`.
pub(crate) fn result_residual(u: f64, outcome: Outcome) -> f64 {
    let (p, _) = shares(u);
    let mean = p[1] + 2.0 * p[2];
    let variance = p[1] + 4.0 * p[2] - mean * mean;
    (f64::from(score(outcome)) - mean).powi(2) / variance.max(1e-12)
}

/// A measure's `−log P(z)` at `u` (but its constant), in `u`.
pub(crate) fn measure(u: f64, z: f64) -> Term {
    let s2 = MEASURE_SPREAD * MEASURE_SPREAD;
    ((z - u).powi(2) / (2.0 * s2), (u - z) / s2, 1.0 / s2)
}

/// A measure's squared Pearson residual at `u`.
pub(crate) fn measure_residual(u: f64, z: f64) -> f64 {
    ((z - u) / MEASURE_SPREAD).powi(2)
}

pub(crate) fn sigmoid(t: f64) -> f64 {
    if t >= 0.0 {
        1.0 / (1.0 + (-t).exp())
    } else {
        let e = t.exp();
        e / (1.0 + e)
    }
}

/// `ln(1 + e^t)` without overflow.
fn softplus(t: f64) -> f64 {
    t.max(0.0) + (-t.abs()).exp().ln_1p()
}

/// A pair's `−log P(better)` at `v`, in `v`.
pub(crate) fn pair(v: f64, better: Better) -> Term {
    match better {
        Better::Newer => {
            let (a, b) = (sigmoid(v - TIE), sigmoid(TIE - v));
            (softplus(TIE - v), -b, a * b)
        }
        Better::Older => {
            let (a, b) = (sigmoid(v + TIE), sigmoid(-v - TIE));
            (softplus(v + TIE), a, a * b)
        }
        Better::Alike => {
            let (p1, p2) = (sigmoid(TIE - v), sigmoid(-TIE - v));
            let p = (p1 - p2).max(1e-300);
            let d = -p1 * (1.0 - p1) + p2 * (1.0 - p2);
            let dd = p1 * (1.0 - p1) * (1.0 - 2.0 * p1) - p2 * (1.0 - p2) * (1.0 - 2.0 * p2);
            (-p.ln(), -d / p, -dd / p + (d / p).powi(2))
        }
    }
}

/// A pair's squared Pearson residual at `v`, its answer scored −1 older, 0 alike, +1 newer.
pub(crate) fn pair_residual(v: f64, better: Better) -> f64 {
    let (newer, older) = (sigmoid(v - TIE), sigmoid(-v - TIE));
    let mean = newer - older;
    let variance = newer + older - mean * mean;
    let said = match better {
        Better::Older => -1.0,
        Better::Alike => 0.0,
        Better::Newer => 1.0,
    };
    (said - mean).powi(2) / variance.max(1e-12)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn derivatives_hold(f: impl Fn(f64) -> Term, at: f64) {
        let e = 1e-5;
        let (_, d1, d2) = f(at);
        let num1 = (f(at + e).0 - f(at - e).0) / (2.0 * e);
        let num2 = (f(at + e).1 - f(at - e).1) / (2.0 * e);
        assert!((d1 - num1).abs() < 1e-6, "{at}: {d1} {num1}");
        assert!((d2 - num2).abs() < 1e-5, "{at}: {d2} {num2}");
        assert!(d2 >= 0.0);
    }

    #[test]
    fn the_rating_scale_gives_the_design_s_shares_and_its_derivatives_hold() {
        let p = |u: f64, o: Outcome| (-result(u, o).0).exp();
        assert!((p(0.0, Outcome::Redo) - 0.212).abs() < 1e-3);
        assert!((p(0.0, Outcome::Near) - 0.576).abs() < 1e-3);
        assert!((p(2.0, Outcome::Clean) - 0.72).abs() < 1e-2);
        for outcome in [Outcome::Redo, Outcome::Near, Outcome::Clean] {
            for u in [-3.0, -0.4, 0.0, 1.3, 4.0] {
                derivatives_hold(|u| result(u, outcome), u);
            }
        }
    }

    #[test]
    fn a_pair_s_three_answers_share_one_and_their_derivatives_hold() {
        let all = [Better::Older, Better::Alike, Better::Newer];
        for v in [-4.0, -1.0, 0.0, 0.3, 2.5] {
            let total: f64 = all.map(|b| (-pair(v, b).0).exp()).iter().sum();
            assert!((total - 1.0).abs() < 1e-12, "{v}: {total}");
            for better in all {
                derivatives_hold(|v| pair(v, better), v);
            }
            // The residuals weigh to one on the mean, as the model expects them to.
            let expected: f64 = (all.iter())
                .map(|b| (-pair(v, *b).0).exp() * pair_residual(v, *b))
                .sum();
            assert!((expected - 1.0).abs() < 1e-9, "{v}: {expected}");
        }
        let alike = (-pair(0.0, Better::Alike).0).exp();
        assert!(
            (alike - 0.40).abs() < 0.01,
            "alike {alike} when nothing changed"
        );
        let outcomes = [Outcome::Redo, Outcome::Near, Outcome::Clean];
        let expected: f64 = (outcomes.iter())
            .map(|o| (-result(0.7, *o).0).exp() * result_residual(0.7, *o))
            .sum();
        assert!((expected - 1.0).abs() < 1e-9);
    }
}
