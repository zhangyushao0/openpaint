//! An object's result (S119): drawn by a steady hand on its visible edges and seen curves it is
//! clean; its longest edge drawn beside its place near; its three longest so, redo (marks are
//! counted, and corners two of them share ringed once); an edge not drawn yet, incomplete.

use openpaint_practice::{
    DrawnCopy, ObjectCompared, ObjectTask, Outcome, PEN_WIDTH, Point, Verdict,
};

use crate::exact::objects;
use crate::hand::Manner;
use crate::hands::drawn;

const OFF: f64 = 2.5 * PEN_WIDTH;

fn verdict(task: &ObjectTask, all: Vec<Vec<Point>>) -> Verdict {
    ObjectCompared::of(
        task,
        &DrawnCopy {
            all,
            corrected: None,
        },
    )
    .verdict()
}

/// `task`'s edges, longest first.
fn longest(task: &ObjectTask) -> Vec<usize> {
    let sides = &task.answer.sides;
    let length = |s: usize| openpaint_practice::distance(sides[s][0], sides[s][1]);
    let mut order: Vec<usize> = (0..sides.len()).collect();
    order.sort_by(|i, j| length(*j).total_cmp(&length(*i)));
    order
}

/// `strokes` with the edges `at` of `task` drawn `OFF` beside their place, outward from the box.
fn beside(task: &ObjectTask, mut strokes: Vec<Vec<Point>>, at: &[usize]) -> Vec<Vec<Point>> {
    let middle =
        (task.seen.corners.iter()).fold([0.0; 2], |m, p| [m[0] + p[0] / 8.0, m[1] + p[1] / 8.0]);
    for k in at {
        let [a, b] = task.answer.sides[*k];
        let long = openpaint_practice::distance(a, b);
        let mut across = [(a[1] - b[1]) / long * OFF, (b[0] - a[0]) / long * OFF];
        let out = [
            (a[0] + b[0]) / 2.0 - middle[0],
            (a[1] + b[1]) / 2.0 - middle[1],
        ];
        if across[0] * out[0] + across[1] * out[1] < 0.0 {
            across = across.map(|v| -v);
        }
        strokes[*k] = [a, b]
            .map(|p| [p[0] + across[0], p[1] + across[1]])
            .to_vec();
    }
    strokes
}

#[test]
fn an_object_drawn_true_is_clean_an_edge_off_near_three_redo_an_edge_missing_incomplete() {
    let (mut near, mut redo, mut all) = (0, 0, 0);
    for (k, task) in objects().enumerate() {
        let strokes = drawn(&task, Manner::STEADY, k as u64);
        assert_eq!(
            verdict(&task, strokes.clone()),
            Verdict::App(Outcome::Clean),
            "{k}"
        );
        let order = longest(&task);
        let one = verdict(&task, beside(&task, strokes.clone(), &order[..1]));
        let three = verdict(&task, beside(&task, strokes.clone(), &order[..3]));
        all += 1;
        near += usize::from(one == Verdict::App(Outcome::Near));
        redo += usize::from(three == Verdict::App(Outcome::Redo));
        // Its last edge not drawn yet.
        let mut missing = strokes;
        missing.remove(task.answer.sides.len() - 1);
        assert_eq!(verdict(&task, missing), Verdict::Incomplete, "{k}");
    }
    // Three edges meeting share corners, each ringed once: now and then two marks for three.
    println!("an edge off: near {near} of {all}; three off: redo {redo} of {all}");
    assert_eq!(near, all);
    assert!(redo * 100 >= all * 98, "{redo} of {all}");
}
