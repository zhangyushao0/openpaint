//! The lines of S141 as they are made: a meander bends three times each way in turn; a layered
//! line carries three or four ripples about three pen widths high across a trend bowing well
//! past them, never swinging far off it.

use super::line::{LineTier, drawn};
use crate::maths::path::Point;

/// The direction from `a` to `b`.
fn towards(a: Point, b: Point) -> Point {
    [b[0] - a[0], b[1] - a[1]]
}

fn cross(u: Point, v: Point) -> f64 {
    u[0] * v[1] - u[1] * v[0]
}

/// `values` gathered into runs of one sign (zeros end a run), each run's `fold`.
fn runs(values: &[f64], fold: fn(f64, f64) -> f64) -> Vec<f64> {
    let mut runs: Vec<f64> = Vec::new();
    let mut last = 0.0_f64;
    for &value in values.iter().filter(|v| v.abs() > 1e-12) {
        match runs.last_mut() {
            Some(run) if value.signum() == last.signum() => *run = fold(*run, value),
            _ => runs.push(value),
        }
        last = value;
    }
    runs
}

#[test]
fn a_meander_bends_three_times_each_way_in_turn() {
    for seed in 0..300 {
        let (line, trend) = drawn(LineTier::Meander, seed);
        assert_eq!(line, trend, "no ripples");
        // The turn at each inner point, summed bend by bend.
        let turns: Vec<f64> = (line.windows(3))
            .map(|w| {
                let (u, v) = (towards(w[0], w[1]), towards(w[1], w[2]));
                cross(u, v).atan2(u[0] * v[0] + u[1] * v[1])
            })
            .collect();
        let bends = runs(&turns, |run, turn| run + turn);
        assert_eq!(bends.len(), 3, "{seed}: {bends:?}");
        assert!(bends[0] * bends[1] < 0.0 && bends[1] * bends[2] < 0.0);
        let plain = bends.iter().all(|bend| (0.55..=1.35).contains(&bend.abs()));
        assert!(plain, "{seed}: {bends:?}");
    }
}

#[test]
fn a_layered_line_carries_three_or_four_ripples_well_under_its_trend_s_bend() {
    for seed in 0..300 {
        let (line, trend) = drawn(LineTier::Layered, seed);
        // How far each point lies across the trend, signed: the ripples.
        let last = trend.len() - 1;
        let across: Vec<f64> = (0..=last)
            .map(|i| {
                let along = towards(trend[i.saturating_sub(1)], trend[(i + 1).min(last)]);
                let moved = towards(trend[i], line[i]);
                cross(along, moved) / along[0].hypot(along[1])
            })
            .collect();
        // Each half wave's crest, then each ripple's height from crest to trough.
        let crests = runs(
            &across,
            |run, off| if off.abs() > run.abs() { off } else { run },
        );
        assert!([6, 8].contains(&crests.len()), "{seed}: {crests:?}");
        let heights: Vec<f64> = (crests.chunks(2))
            .map(|pair| pair[0].abs() + pair[1].abs())
            .collect();
        let high = heights.iter().fold(0.0_f64, |a, b| a.max(*b));
        assert!(
            heights.iter().all(|h| (0.03..=0.04).contains(h)),
            "{seed}: {heights:?}"
        );
        // The trend's bend: its farthest from the chord between its ends.
        let chord = towards(trend[0], trend[last]);
        let bend = (trend.iter())
            .map(|p| cross(chord, towards(trend[0], *p)).abs() / chord[0].hypot(chord[1]))
            .fold(0.0_f64, f64::max);
        assert!(bend >= 1.75 * high, "{seed}: bend {bend} ripples {high}");
        // Ripples, not steps or hooks: the line never swings far off its trend.
        let swing = (line.windows(2).zip(trend.windows(2)))
            .map(|(l, t)| {
                let (u, v) = (towards(l[0], l[1]), towards(t[0], t[1]));
                let turn = cross(u, v).atan2(u[0] * v[0] + u[1] * v[1]);
                turn.abs().to_degrees()
            })
            .fold(0.0_f64, f64::max);
        assert!(swing <= 48.0, "{seed}: {swing} degrees");
    }
}
