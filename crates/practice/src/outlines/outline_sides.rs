//! A plain ground's sides (`outline_ground.rs`): each side's colour read along it as a profile,
//! freed of where the object touches the side and smoothed. A place of a side is the object's
//! when it strays from the side's running median (over `MEDIAN_SPAN` of the side) by more than
//! `OFF` noises (and at least `LEAST_OFF`), or from the side's trend by more than `FAR_OFF`: a
//! polynomial of degree `DEGREE` fitted to the `TRIMMED` share of the side nearest it, in
//! `TREND_ROUNDS` (so an object touching more of the side than the median spans is still found).
//! The profile is then smoothed over `SMOOTH` of the side.

use crate::outline_ground::distance;

const MEDIAN_SPAN: f64 = 0.16;
const TRIMMED: f64 = 0.55;
const TREND_ROUNDS: usize = 6;
const DEGREE: usize = 4;
const FAR_OFF: f64 = 0.08;
const OFF: f64 = 4.0;
const LEAST_OFF: f64 = 0.015;
const SMOOTH: f64 = 0.02;

/// A side's profile as the ground's (its places straying replaced by the ground on either side of
/// them, then smoothed), and how many places were ground.
pub(crate) fn profile(side: &[[f64; 3]]) -> (Vec<[f64; 3]>, usize) {
    let length = side.len();
    let reach = ((MEDIAN_SPAN * length as f64 / 2.0) as usize).max(2);
    let median: Vec<[f64; 3]> = (0..length)
        .map(|at| {
            let window = &side[at.saturating_sub(reach)..(at + reach + 1).min(length)];
            [0, 1, 2].map(|c| {
                let mut values: Vec<f64> = window.iter().map(|v| v[c]).collect();
                values.sort_by(f64::total_cmp);
                values[values.len() / 2]
            })
        })
        .collect();
    let trend = trend(side);
    let gaps: Vec<f64> = (0..length)
        .map(|at| distance(side[at], median[at]))
        .collect();
    let mut sorted = gaps.clone();
    sorted.sort_by(f64::total_cmp);
    let limit = (OFF * 1.4826 * sorted[length / 2]).max(LEAST_OFF);
    let ground: Vec<bool> = (0..length)
        .map(|at| gaps[at] <= limit && distance(side[at], trend[at]) <= FAR_OFF)
        .collect();
    let kept = ground.iter().filter(|g| **g).count();
    if kept == 0 {
        return (median, 0);
    }
    // Strayed places take the ground's value between the nearest ground places on either side.
    let mut filled = side.to_vec();
    let known: Vec<usize> = (0..length).filter(|&at| ground[at]).collect();
    for at in (0..length).filter(|&at| !ground[at]) {
        let after = known.partition_point(|&k| k < at);
        filled[at] = match (after.checked_sub(1).map(|b| known[b]), known.get(after)) {
            (Some(before), Some(&next)) => {
                let t = (at - before) as f64 / (next - before) as f64;
                [0, 1, 2].map(|c| side[before][c] + t * (side[next][c] - side[before][c]))
            }
            (Some(before), None) => side[before],
            (None, Some(&next)) => side[next],
            (None, None) => median[at],
        };
    }
    let smooth = ((SMOOTH * length as f64 / 2.0) as usize).max(1);
    let smoothed = (0..length)
        .map(|at| {
            let window = &filled[at.saturating_sub(smooth)..(at + smooth + 1).min(length)];
            [0, 1, 2].map(|c| window.iter().map(|v| v[c]).sum::<f64>() / window.len() as f64)
        })
        .collect();
    (smoothed, kept)
}

/// A side's trend: a polynomial along it per channel, fitted again and again to the `TRIMMED`
/// share of its places that lie nearest the fit before (least trimmed squares), from the side's
/// median colour, so an object touching up to almost half the side does not bend it.
fn trend(side: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let length = side.len();
    let place = |at: usize| at as f64 / (length - 1).max(1) as f64 - 0.5;
    let median = [0, 1, 2].map(|c| {
        let mut values: Vec<f64> = side.iter().map(|v| v[c]).collect();
        values.sort_by(f64::total_cmp);
        values[length / 2]
    });
    let mut fitted = vec![median; length];
    for _ in 0..TREND_ROUNDS {
        let mut kept: Vec<usize> = (0..length).collect();
        kept.sort_by(|&a, &b| {
            distance(side[a], fitted[a]).total_cmp(&distance(side[b], fitted[b]))
        });
        kept.truncate(((TRIMMED * length as f64) as usize).max(3));
        let terms = [0, 1, 2].map(|c| polynomial(&kept, place, |at| side[at][c]));
        fitted = (0..length)
            .map(|at| {
                let t = place(at);
                terms.map(|terms| terms.iter().rev().fold(0.0, |sum, term| sum * t + term))
            })
            .collect();
    }
    fitted
}

/// The polynomial of degree `DEGREE` nearest `value` at the places `kept` (least squares), its
/// terms from the constant up; the mean where the places do not fix it.
fn polynomial(
    kept: &[usize],
    place: impl Fn(usize) -> f64,
    value: impl Fn(usize) -> f64,
) -> [f64; DEGREE + 1] {
    const TERMS: usize = DEGREE + 1;
    let mut sums = [0.0f64; 2 * DEGREE + 1];
    let mut sides = [0.0f64; TERMS];
    for &at in kept {
        let (t, v) = (place(at), value(at));
        let mut power = 1.0;
        for (k, sum) in sums.iter_mut().enumerate() {
            *sum += power;
            if let Some(side) = sides.get_mut(k) {
                *side += power * v;
            }
            power *= t;
        }
    }
    let mut matrix = [[0.0f64; TERMS]; TERMS];
    for (row, line) in matrix.iter_mut().enumerate() {
        for (column, cell) in line.iter_mut().enumerate() {
            *cell = sums[row + column];
        }
    }
    let mut mean = [0.0; TERMS];
    mean[0] = sides[0] / sums[0].max(1.0);
    // Gaussian elimination with partial pivoting.
    for column in 0..TERMS {
        let Some(pivot) = (column..TERMS)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))
        else {
            return mean;
        };
        if matrix[pivot][column].abs() < 1e-12 {
            return mean;
        }
        matrix.swap(column, pivot);
        sides.swap(column, pivot);
        let above = matrix[column];
        for row in column + 1..TERMS {
            let factor = matrix[row][column] / above[column];
            for (cell, over) in matrix[row].iter_mut().zip(above).skip(column) {
                *cell -= factor * over;
            }
            sides[row] -= factor * sides[column];
        }
    }
    let mut terms = [0.0; TERMS];
    for row in (0..TERMS).rev() {
        let known: f64 = (row + 1..TERMS).map(|k| matrix[row][k] * terms[k]).sum();
        terms[row] = (sides[row] - known) / matrix[row][row];
    }
    terms
}
