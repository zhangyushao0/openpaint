//! A solid's copy read back (S93): simulated hands (S84's steady and shaky manners) drawing each
//! solid as V大 does, its ellipses round and through (a cylinder's far end whole, its hidden half
//! too) and its sides ruled, read back as its ellipses and sides; how far round an ellipse must go
//! before it is fitted; a correction read in place of what it redraws; nothing read where nothing
//! was drawn near the solid.

use openpaint_practice::{
    COVERED, Ellipse, Metric, Point, Random, Solid, SolidCheck, SolidTask, distance, length, value,
};

use crate::hand::{Hand, Manner};

const SEEDS: u64 = 100;

/// `e` drawn round by a hand in `manner`: from a turn of its own, a little past once round,
/// wobbling slowly and trembling across the outline; `share` of the way round when less.
pub(crate) fn round(
    e: &Ellipse,
    (random, manner): (&mut Random, Manner),
    share: f64,
) -> Vec<Point> {
    let start = random.range(0.0, std::f64::consts::TAU);
    let turns = if share < 1.0 {
        share
    } else {
        random.range(1.02, 1.12)
    };
    let (waves, phase) = (random.range(1.0, 3.0), random.range(0.0, 6.3));
    let wobble = random.range(0.0, manner.wobble);
    let steps = (240.0 * turns) as usize;
    (0..=steps)
        .map(|k| {
            let s = k as f64 / steps as f64;
            let t = start + s * turns * std::f64::consts::TAU;
            let [x, y] = e.at(t);
            let [cx, cy] = e.centre;
            let long = (x - cx).hypot(y - cy).max(1e-9);
            let out = wobble * (std::f64::consts::TAU * waves * s + phase).sin()
                + random.range(-manner.tremor, manner.tremor);
            [x + out * (x - cx) / long, y + out * (y - cy) / long]
        })
        .collect()
}

/// `task`'s solid drawn by a hand in `manner` seeded `seed`: each ellipse round once (V大's
/// circle for a sphere), each side ruled.
fn drawn(task: &SolidTask, manner: Manner, seed: u64) -> Vec<Vec<Point>> {
    let mut random = Random::new(seed);
    let mut hand = Hand::new(seed ^ 0x5011D, manner);
    let mut strokes: Vec<Vec<Point>> = Vec::new();
    let ellipses = match task.circle() {
        Some((centre, radius)) => vec![Ellipse {
            centre,
            axes: [radius; 2],
            angle: 0.0,
        }],
        None => task.answer.ellipses.clone(),
    };
    for e in &ellipses {
        strokes.push(round(e, (&mut random, manner), 1.0));
    }
    for [a, b] in &task.answer.sides {
        strokes.push(hand.line(*a, *b));
    }
    strokes
}

fn lines(strokes: &[Vec<Point>]) -> Vec<&[Point]> {
    strokes.iter().map(Vec::as_slice).collect()
}

/// Whether `fitted` is `exact` as near as a hand draws it: its centre within 1.2 % of the frame,
/// its axes within 8 %.
fn near(fitted: &Ellipse, exact: &Ellipse) -> bool {
    distance(fitted.centre, exact.centre) <= 0.012
        && (0..2).all(|k| (fitted.axes[k] - exact.axes[k]).abs() <= 0.08 * exact.axes[k])
}

/// Over `SEEDS` tasks of `solid` drawn in `manner`: the share of ellipses read and fitted near the
/// exact one, of sides read, and the worst error kept (a share of the box).
fn read_back(solid: Solid, manner: Manner) -> (f64, f64, f64) {
    let (mut fitted, mut ellipses, mut sides, mut all_sides, mut worst) = (0, 0, 0, 0, 0.0_f64);
    for seed in 0..SEEDS {
        let task = SolidTask::generate(solid, seed + 1000);
        let strokes = drawn(&task, manner, seed);
        let check = SolidCheck::of(&task, (&lines(&strokes), &[]));
        for (curve, exact) in check.curves.iter().zip(&task.answer.ellipses) {
            ellipses += 1;
            fitted += usize::from(curve.fitted.is_some_and(|f| near(&f, exact)));
        }
        for side in &check.sides {
            all_sides += 1;
            sides += usize::from(!side.is_empty());
        }
        worst = worst.max(value(&check.metrics, Metric::SolidOff).unwrap_or(1.0));
    }
    let share = |n: usize, of: usize| if of == 0 { 1.0 } else { n as f64 / of as f64 };
    (share(fitted, ellipses), share(sides, all_sides), worst)
}

#[test]
fn simulated_hands_drawing_each_solid_read_back_as_its_ellipses_and_sides() {
    for solid in Solid::ALL {
        let (fitted, sides, worst) = read_back(solid, Manner::STEADY);
        println!("{solid:?} steady: ellipses {fitted:.3}, sides {sides:.3}, worst off {worst:.4}");
        assert!(fitted >= 0.98 && sides >= 0.99, "{solid:?} steady");
        assert!(worst < 0.03, "{solid:?} steady: {worst}");
        let (fitted, sides, worst) = read_back(solid, Manner::SHAKY);
        println!("{solid:?} shaky: ellipses {fitted:.3}, sides {sides:.3}, worst off {worst:.4}");
        assert!(fitted >= 0.93 && sides >= 0.97, "{solid:?} shaky");
    }
}

#[test]
fn an_ellipse_is_fitted_once_drawn_two_thirds_round_and_shown_as_drawn_before() {
    let task = SolidTask::generate(Solid::Cylinder, 7);
    let far = task.answer.ellipses[1];
    let manner = Manner::STEADY;
    let mut random = Random::new(3);
    for (share, fits) in [(1.0, true), (0.75, true), (0.5, false), (0.25, false)] {
        let arc = round(&far, (&mut random, manner), share);
        let check = SolidCheck::of(&task, (&[&arc], &[]));
        let curve = &check.curves[1];
        assert_eq!(curve.fitted.is_some(), fits, "{share}");
        // Shown as drawn, but for a little where it touches a side or passes the near end.
        let long = |pieces: &[Vec<Point>]| pieces.iter().map(|p| length(p)).sum::<f64>();
        let read = long(&curve.pieces);
        assert!(read >= 0.85 * length(&arc), "{share}: {read}");
        let off = value(&check.metrics, Metric::SolidOff).expect("an error kept");
        assert!(off < 0.02, "{share}: {off}");
    }
    assert!((COVERED - 2.0 / 3.0).abs() < 1e-12);
}

#[test]
fn a_correction_is_read_in_place_of_what_it_redraws_and_nothing_far_is_read() {
    let task = SolidTask::generate(Solid::Cone, 11);
    let base = task.answer.ellipses[0];
    let wide = Ellipse {
        axes: [base.axes[0] * 1.15, base.axes[1] * 1.15],
        ..base
    };
    let mut random = Random::new(5);
    let first = round(&wide, (&mut random, Manner::STEADY), 1.0);
    let mut hand = Hand::new(9, Manner::STEADY);
    let [a, b] = task.answer.sides[0];
    let side = hand.line(a, b);
    let drawn = [first.as_slice(), side.as_slice()];
    let before = SolidCheck::of(&task, (&drawn, &[]));
    let fitted = before.curves[0].fitted.expect("the first try fitted");
    assert!(!near(&fitted, &base), "the first try too wide");
    let again = round(&base, (&mut random, Manner::STEADY), 1.0);
    let after = SolidCheck::of(&task, (&drawn, &[again.as_slice()]));
    let fitted = after.curves[0].fitted.expect("the correction fitted");
    assert!(near(&fitted, &base), "the correction alone");
    // The side drawn first still counts whole: the correction only passes where it touches.
    let side: f64 = after.sides[0].iter().map(|piece| length(piece)).sum();
    assert!(side > 0.8 * distance(a, b), "{side}");
    let off = |check: &SolidCheck| value(&check.metrics, Metric::SolidOff).unwrap_or(1.0);
    assert!(off(&after) < off(&before));
    // A scribble in a corner of the frame, far from the solid, is read as nothing.
    let corner: Vec<Point> = (0..30).map(|k| [0.01 + 0.001 * k as f64, 0.01]).collect();
    let nothing = SolidCheck::of(&task, (&[corner.as_slice()], &[]));
    assert!(nothing.metrics.is_empty());
    assert!(
        nothing
            .curves
            .iter()
            .all(|c| c.pieces.is_empty() && c.fitted.is_none())
    );
}
