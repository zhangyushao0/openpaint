//! A solid's result (S119): drawn by a steady hand on its answer it is clean; a side drawn beside
//! its place (its touches ringed, or run on) near; an end drawn wide (its middles ringed) redo; a
//! side not drawn yet, incomplete.

use openpaint_practice::{
    DrawnCopy, Outcome, PEN_WIDTH, Point, Random, Solid, SolidCompared, SolidTask, Verdict,
};

use crate::hand::{Hand, Manner};
use crate::hands::round;

/// How far a part is drawn off on purpose: two and a half pen widths.
const OFF: f64 = 2.5 * PEN_WIDTH;

/// `task`'s solid drawn by a steady hand: its ellipses round once each, `ends` scaled out by
/// `wider`, its sides ruled (the first moved by `side`), the last `missing` sides left out.
fn drawn(task: &SolidTask, (wider, side): (f64, Point), missing: usize, seed: u64) -> Verdict {
    let mut random = Random::new(seed);
    let mut hand = Hand::new(seed ^ 0x5011D, Manner::STEADY);
    let mut strokes: Vec<Vec<Point>> = (task.answer.ellipses.iter())
        .map(|e| {
            let mut e = *e;
            e.axes = e.axes.map(|a| a + wider);
            round(&e, (&mut random, Manner::STEADY), 1.0)
        })
        .collect();
    let sides = task.answer.sides.len() - missing;
    for (k, [a, b]) in task.answer.sides[..sides].iter().enumerate() {
        let by = if k == 0 { side } else { [0.0; 2] };
        strokes.push(hand.line([a[0] + by[0], a[1] + by[1]], [b[0] + by[0], b[1] + by[1]]));
    }
    SolidCompared::of(
        task,
        &DrawnCopy {
            all: strokes,
            corrected: None,
        },
    )
    .verdict()
}

#[test]
fn a_cylinder_drawn_true_is_clean_a_side_off_near_an_end_wide_redo_a_side_missing_incomplete() {
    for seed in 0..20 {
        let task = SolidTask::generate(Solid::Cylinder, seed + 1000);
        assert_eq!(
            drawn(&task, (0.0, [0.0; 2]), 0, seed),
            Verdict::App(Outcome::Clean)
        );
        // The first side drawn outward, away from the other.
        let [a, b] = task.answer.sides[0];
        let [c, d] = task.answer.sides[1];
        let long = openpaint_practice::distance(a, b);
        let mut across = [(a[1] - b[1]) / long * OFF, (b[0] - a[0]) / long * OFF];
        let other = [
            (c[0] + d[0] - a[0] - b[0]) / 2.0,
            (c[1] + d[1] - a[1] - b[1]) / 2.0,
        ];
        if across[0] * other[0] + across[1] * other[1] > 0.0 {
            across = across.map(|v| -v);
        }
        assert_eq!(
            drawn(&task, (0.0, across), 0, seed),
            Verdict::App(Outcome::Near),
            "{seed}"
        );
        assert_eq!(
            drawn(&task, (OFF, [0.0; 2]), 0, seed),
            Verdict::App(Outcome::Redo),
            "{seed}"
        );
        assert_eq!(
            drawn(&task, (0.0, [0.0; 2]), 1, seed),
            Verdict::Incomplete,
            "{seed}"
        );
    }
}
