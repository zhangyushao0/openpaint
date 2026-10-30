//! An object's copy read back (S94): simulated hands (S84's steady and shaky manners) drawing each
//! object's visible edges as strokes and its round part's seen curves round, read back as its
//! edges and curves; a correction read in place of what it redraws; nothing read far from it.

use openpaint_practice::{Metric, ObjectTask, Point, Random, SolidCheck, distance, length, value};

use crate::exact::objects;
use crate::hand::{Hand, Manner};

/// `points` as a hand draws a curve: trembling across it, wobbling slowly.
fn curve(points: &[Point], random: &mut Random, manner: Manner) -> Vec<Point> {
    let (waves, phase) = (random.range(1.0, 3.0), random.range(0.0, 6.3));
    let wobble = random.range(0.0, manner.wobble);
    let last = points.len().saturating_sub(1).max(1) as f64;
    (points.iter().enumerate())
        .map(|(k, p)| {
            let s = k as f64 / last;
            let out = wobble * (std::f64::consts::TAU * waves * s + phase).sin()
                + random.range(-manner.tremor, manner.tremor);
            let (before, after) = (
                points[k.saturating_sub(1)],
                points[(k + 1).min(points.len() - 1)],
            );
            let d = [after[0] - before[0], after[1] - before[1]];
            let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
            [p[0] - d[1] / long * out, p[1] + d[0] / long * out]
        })
        .collect()
}

/// `task`'s visible edges drawn by a hand in `manner` seeded `seed`, each a stroke, and its seen
/// curves.
fn drawn(task: &ObjectTask, manner: Manner, seed: u64) -> Vec<Vec<Point>> {
    let mut hand = Hand::new(seed, manner);
    let mut random = Random::new(seed ^ 0x0B1E);
    let mut strokes: Vec<Vec<Point>> = (task.answer.sides.iter())
        .map(|[a, b]| hand.line(*a, *b))
        .collect();
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    strokes.extend(seen.map(|stretch| curve(&stretch.points, &mut random, manner)));
    strokes
}

fn lines(strokes: &[Vec<Point>]) -> Vec<&[Point]> {
    strokes.iter().map(Vec::as_slice).collect()
}

/// Over every test object drawn in `manner`: the share of visible edges and of seen curves read
/// back, and the worst error kept (a share of the box).
fn read_back(manner: Manner) -> (f64, f64, f64) {
    let (mut sides, mut all_sides, mut curves, mut all_curves, mut worst) = (0, 0, 0, 0, 0.0_f64);
    for (k, task) in objects().enumerate() {
        let strokes = drawn(&task, manner, k as u64);
        let check = SolidCheck::of_object(&task, (&lines(&strokes), &[]));
        for side in &check.sides {
            all_sides += 1;
            sides += usize::from(!side.is_empty());
        }
        for (read, stretches) in check.curves.iter().zip(&task.answer.outlines) {
            if stretches.iter().any(|stretch| !stretch.hidden) {
                all_curves += 1;
                curves += usize::from(!read.pieces.is_empty());
            }
        }
        worst = worst.max(value(&check.metrics, Metric::ObjectOff).unwrap_or(1.0));
    }
    let share = |n: usize, of: usize| if of == 0 { 1.0 } else { n as f64 / of as f64 };
    (share(sides, all_sides), share(curves, all_curves), worst)
}

#[test]
fn simulated_hands_drawing_each_object_read_back_every_visible_edge() {
    let (sides, curves, worst) = read_back(Manner::STEADY);
    println!("steady: edges {sides:.4}, curves {curves:.4}, worst off {worst:.4}");
    assert!(sides >= 0.999 && curves >= 0.99, "steady");
    assert!(worst < 0.02, "steady: {worst}");
    let (sides, curves, worst) = read_back(Manner::SHAKY);
    println!("shaky: edges {sides:.4}, curves {curves:.4}, worst off {worst:.4}");
    assert!(sides >= 0.99 && curves >= 0.98, "shaky");
    assert!(worst < 0.03, "shaky: {worst}");
}

#[test]
fn a_correction_is_read_in_place_of_what_it_redraws_and_nothing_far_is_read() {
    let task = objects().next().expect("an object");
    let mut hand = Hand::new(5, Manner::STEADY);
    let [a, b] = task.answer.sides[0];
    // The first edge drawn 3 % of the frame off, the others along their answer.
    let shift = |p: Point| [p[0] + 0.03, p[1] + 0.02];
    let mut first = vec![hand.line(shift(a), shift(b))];
    first.extend((task.answer.sides[1..].iter()).map(|[p, q]| hand.line(*p, *q)));
    let before = SolidCheck::of_object(&task, (&lines(&first), &[]));
    let off = |check: &SolidCheck| value(&check.metrics, Metric::ObjectOff).unwrap_or(1.0);
    let again = hand.line(a, b);
    let after = SolidCheck::of_object(&task, (&lines(&first), &[again.as_slice()]));
    assert!(
        off(&after) < off(&before),
        "{} -> {}",
        off(&before),
        off(&after)
    );
    let redrawn: f64 = after.sides[0].iter().map(|piece| length(piece)).sum();
    assert!(
        (redrawn - distance(a, b)).abs() < 0.3 * distance(a, b),
        "the correction alone"
    );
    // A scribble in a corner of the frame, far from the object, is read as nothing.
    let corner: Vec<Point> = (0..30)
        .map(|k| [0.01 + 0.001 * f64::from(k), 0.01])
        .collect();
    let nothing = SolidCheck::of_object(&task, (&[corner.as_slice()], &[]));
    assert!(nothing.metrics.is_empty());
    assert!(nothing.sides.iter().all(Vec::is_empty));
}
