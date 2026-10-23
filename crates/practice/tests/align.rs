//! The alignment on known cases: a copy moved and evenly sized from its original gives back that
//! shift and scale, also drawn in pieces, shaky, overshooting or with a stray mark; a turned
//! copy is never turned back; a copy with no length gives no fit.

use openpaint_practice::{Fit, LineExercise, LineTier, Point, align, distance};

fn line(tier: LineTier, seed: u64) -> Vec<Point> {
    LineExercise::generate(tier, seed).points
}

fn moved(points: &[Point], fit: Fit) -> Vec<Point> {
    points.iter().map(|p| fit.apply(*p)).collect()
}

/// Whether `found` lays `original` within `tolerance` (frame sides) of where `known` does.
fn recovers(original: &[Point], found: Fit, known: Fit, tolerance: f64) -> bool {
    original
        .iter()
        .all(|p| distance(found.apply(*p), known.apply(*p)) <= tolerance)
}

const KNOWN: [Fit; 4] = [
    Fit {
        scale: 1.0,
        shift: [0.08, -0.05],
    },
    Fit {
        scale: 0.7,
        shift: [0.15, 0.2],
    },
    Fit {
        scale: 1.3,
        shift: [-0.12, -0.1],
    },
    Fit {
        scale: 0.5,
        shift: [0.3, 0.1],
    },
];

#[test]
fn a_moved_and_sized_copy_gives_back_its_shift_and_scale() {
    for tier in LineTier::ALL {
        let original = line(tier, 7);
        for known in KNOWN {
            let copy = moved(&original, known);
            let found = align(&original, &[&copy]).expect("a fit");
            assert!(
                (found.scale - known.scale).abs() < 1e-6,
                "{tier:?} {found:?}"
            );
            assert!(
                distance(found.shift, known.shift) < 1e-6,
                "{tier:?} {found:?}"
            );
        }
    }
}

#[test]
fn a_copy_in_pieces_with_a_gap_still_gives_them_back() {
    let original = line(LineTier::Arc, 11);
    let known = KNOWN[1];
    let copy = moved(&original, known);
    // Three strokes, a little of the line left out between two of them.
    let pieces = [&copy[..150], &copy[140..300], &copy[320..]];
    let found = align(&original, &pieces).expect("a fit");
    assert!(recovers(&original, found, known, 0.004), "{found:?}");
}

#[test]
fn a_shaky_copy_lands_its_original_on_the_line_the_hand_meant() {
    let original = line(LineTier::Wave, 3);
    let known = KNOWN[2];
    // A tremor across the line: 0.6 % of the frame either way, a wave every 2 % of its points.
    let shaky: Vec<Point> = moved(&original, known)
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let side = 0.006 * (i as f64 * std::f64::consts::TAU / 10.0).sin();
            [p[0] + side, p[1] - side]
        })
        .collect();
    let found = align(&original, &[&shaky]).expect("a fit");
    // Within the tremor's own reach (0.85 % of the frame across the line).
    assert!(recovers(&original, found, known, 0.01), "{found:?}");
}

#[test]
fn an_overshoot_and_a_stray_mark_do_not_pull_the_fit() {
    let original = line(LineTier::Hook, 5);
    let known = KNOWN[1];
    let copy = moved(&original, known);
    // The line run on past its end a fifth of its length, and a tick far from it.
    let (a, b) = (copy[copy.len() - 2], copy[copy.len() - 1]);
    let run: Vec<Point> = (0..=40)
        .map(|k| {
            let t = f64::from(k) * 2.0;
            [b[0] + (b[0] - a[0]) * t, b[1] + (b[1] - a[1]) * t]
        })
        .collect();
    let tick: Vec<Point> = vec![[0.9, 0.9], [0.92, 0.88]];
    let found = align(&original, &[&copy, &run, &tick]).expect("a fit");
    assert!(recovers(&original, found, known, 0.02), "{found:?}");
}

#[test]
fn a_turned_copy_is_moved_and_sized_but_never_turned() {
    let original = line(LineTier::Straight, 9);
    let (sin, cos) = 0.5_f64.sin_cos();
    let turned: Vec<Point> = original
        .iter()
        .map(|p| {
            let [x, y] = [p[0] - 0.5, p[1] - 0.5];
            [0.5 + x * cos - y * sin, 0.5 + x * sin + y * cos]
        })
        .collect();
    let found = align(&original, &[&turned]).expect("a fit");
    // The same length, so about the same size; the angle stays in the overlay.
    assert!((found.scale - 1.0).abs() < 0.15, "{found:?}");
    let ends = [found.apply(original[0]), found.apply(original[480])];
    assert!(distance(ends[0], turned[0]) > 0.03, "{ends:?}");
}

#[test]
fn a_copy_with_no_length_gives_no_fit() {
    let original = line(LineTier::Arc, 1);
    assert_eq!(align(&original, &[]), None);
    assert_eq!(align(&original, &[&[[0.5, 0.5]]]), None);
    assert_eq!(align(&original, &[&[[0.5, 0.5], [0.5, 0.5]]]), None);
}
