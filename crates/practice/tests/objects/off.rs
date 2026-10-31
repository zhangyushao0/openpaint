//! What is off on an object's copy (S98): simulated hands (S84's steady and shaky manners) drawing
//! every test object's visible edges and seen curves get no mark; an edge drawn two and a half pen
//! widths beside its place, away from the object, is marked at it alone: its corners ringed, or,
//! turned from the vanishing point it runs to, run on past it.

use openpaint_practice::{Off, PEN_WIDTH, Point, SolidCheck, distance};

use crate::exact::objects;
use crate::hand::Manner;
use crate::hands::drawn;

const OFF: f64 = 2.5 * PEN_WIDTH;

fn check(task: &openpaint_practice::ObjectTask, strokes: &[Vec<Point>]) -> SolidCheck {
    let lines: Vec<&[Point]> = strokes.iter().map(Vec::as_slice).collect();
    SolidCheck::of_object(task, (&lines, &[]))
}

#[test]
fn objects_drawn_on_their_answer_by_steady_and_shaky_hands_get_no_mark() {
    for manner in [Manner::STEADY, Manner::SHAKY] {
        let (mut marked, mut all) = (0, 0);
        for (k, task) in objects().enumerate() {
            let read = check(&task, &drawn(&task, manner, k as u64));
            all += 1;
            marked += usize::from(!read.off.is_empty());
        }
        println!("{:?}: {marked} of {all} marked", manner.tremor);
        assert_eq!(marked, 0);
    }
}

#[test]
fn an_edge_beside_its_place_is_marked_at_it_alone() {
    let mut run_on = 0;
    for (k, task) in objects().enumerate() {
        // Its longest visible edge, drawn outward from the box's middle.
        let sides = &task.answer.sides;
        let Some(at) = (0..sides.len()).max_by(|i, j| {
            let length = |s: usize| distance(sides[s][0], sides[s][1]);
            length(*i).total_cmp(&length(*j))
        }) else {
            continue;
        };
        let [a, b] = sides[at];
        let long = distance(a, b);
        let middle = task
            .seen
            .corners
            .iter()
            .fold([0.0; 2], |m, p| [m[0] + p[0] / 8.0, m[1] + p[1] / 8.0]);
        let mut across = [(a[1] - b[1]) / long * OFF, (b[0] - a[0]) / long * OFF];
        let out = [
            (a[0] + b[0]) / 2.0 - middle[0],
            (a[1] + b[1]) / 2.0 - middle[1],
        ];
        if across[0] * out[0] + across[1] * out[1] < 0.0 {
            across = across.map(|v| -v);
        }
        let mut strokes = drawn(&task, Manner::STEADY, k as u64);
        let shifted = [a, b].map(|p| [p[0] + across[0], p[1] + across[1]]);
        strokes[at] = shifted.to_vec();
        let read = check(&task, &strokes);
        assert!(!read.off.is_empty(), "{k}");
        for mark in &read.off {
            match mark {
                Off::Ring(p) => {
                    let corner = [a, b].iter().any(|q| distance(*p, *q) < 1e-9);
                    assert!(corner, "{k}: {p:?} not at {a:?} {b:?}");
                }
                Off::RunOn { line, .. } => {
                    run_on += 1;
                    let from = shifted.iter().map(|q| distance(line[0], *q));
                    assert!(from.fold(1.0, f64::min) < 1e-6, "{k}: {line:?}");
                }
                Off::Plumb(_) => panic!("{k}: an upright drawn upright leans"),
            }
        }
    }
    println!("run on: {run_on}");
}
