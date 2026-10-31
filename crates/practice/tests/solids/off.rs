//! What is off on a solid's copy (S98): simulated hands (S84's steady and shaky manners) drawing
//! each solid's exact ellipses round and its sides ruled get no mark; an end drawn more than a
//! pen's width wide has the middles it runs past ringed, and nothing else; a side drawn beside its
//! place has its touches ringed (or, turned off where it heads, is run on); a cone's side aimed
//! beside its point is run on past it; a sphere's outline drawn wide has its axes' ends ringed.

use openpaint_practice::{
    Ellipse, Off, PEN_WIDTH, Point, Random, Solid, SolidCheck, SolidTask, distance,
};

use crate::hand::{Hand, Manner};
use crate::hands::round;

const SEEDS: u64 = 100;
/// How far a part is drawn off on purpose: two and a half pen widths.
const OFF: f64 = 2.5 * PEN_WIDTH;
/// A cone's side read apart from its base: 80 pixels of a frame.
const CONE_SIDE: f64 = 0.2;

/// `task`'s solid drawn by a hand in `manner`: `ends` round once each, its sides ruled.
fn drawn(task: &SolidTask, (manner, seed): (Manner, u64), ends: &[Ellipse]) -> Vec<Vec<Point>> {
    let mut random = Random::new(seed);
    let mut hand = Hand::new(seed ^ 0x5011D, manner);
    let mut strokes: Vec<Vec<Point>> = (ends.iter())
        .map(|e| round(e, (&mut random, manner), 1.0))
        .collect();
    strokes.extend(task.answer.sides.iter().map(|[a, b]| hand.line(*a, *b)));
    strokes
}

fn check(task: &SolidTask, strokes: &[Vec<Point>]) -> SolidCheck {
    let lines: Vec<&[Point]> = strokes.iter().map(Vec::as_slice).collect();
    SolidCheck::of(task, (&lines, &[]))
}

fn near(p: Point, among: &[Point]) -> bool {
    among.iter().any(|q| distance(p, *q) < 1e-9)
}

#[test]
fn solids_drawn_on_their_answer_by_steady_and_shaky_hands_get_no_mark() {
    for solid in Solid::ALL {
        for manner in [Manner::STEADY, Manner::SHAKY] {
            let marked = (0..SEEDS)
                .filter(|seed| {
                    let task = SolidTask::generate(solid, seed + 1000);
                    let off = check(&task, &drawn(&task, (manner, *seed), &task.answer.ellipses));
                    !off.off.is_empty()
                })
                .count();
            println!("{solid:?} {:?}: {marked} of {SEEDS} marked", manner.tremor);
            assert_eq!(marked, 0, "{solid:?}");
        }
    }
}

#[test]
fn an_end_drawn_wide_has_the_middles_it_runs_past_ringed_and_nothing_else() {
    let mut far = 0;
    for seed in 0..SEEDS {
        let task = SolidTask::generate(Solid::Cylinder, seed + 1000);
        let mut ends = task.answer.ellipses.clone();
        ends[0].axes = ends[0].axes.map(|a| a + OFF);
        let read = check(&task, &drawn(&task, (Manner::STEADY, seed), &ends));
        let [near_end, far_end] = [0, 1].map(|k| task.squares[k].touches);
        let ringed: Vec<Point> = (read.off.iter())
            .map(|mark| match mark {
                Off::Ring(p) => *p,
                _ => panic!("{seed}: {mark:?}"),
            })
            .collect();
        assert!(ringed.iter().any(|p| near(*p, &near_end)), "{seed}");
        assert!(
            ringed
                .iter()
                .all(|p| near(*p, &near_end) || near(*p, &far_end))
        );
        // Where the ends lie close, a stretch of the wide end lies nearer the far one and is
        // read for it (S93's reading: each stretch to the part nearest it).
        far += usize::from(ringed.iter().any(|p| near(*p, &far_end)));
        // A sphere's outline drawn wide: its axes' ends.
        let task = SolidTask::generate(Solid::Sphere, seed + 1000);
        let mut outline = task.answer.ellipses[0];
        outline.axes = outline.axes.map(|a| a + OFF);
        let read = check(&task, &drawn(&task, (Manner::STEADY, seed), &[outline]));
        let exact = task.answer.ellipses[0];
        let ends: Vec<Point> = (0..4)
            .map(|k| exact.at(f64::from(k) * std::f64::consts::FRAC_PI_2))
            .collect();
        assert_eq!(read.off.len(), 4, "{seed}: {:?}", read.off);
        for mark in &read.off {
            assert!(matches!(mark, Off::Ring(p) if ends.iter().any(|e| distance(*e, *p) < 1e-9)));
        }
    }
    println!("a far end's middle ringed too in {far} of {SEEDS}");
    assert!(far * 10 <= SEEDS as usize, "{far}");
}

#[test]
fn a_side_beside_its_place_is_marked_at_it_and_a_cone_s_aimed_off_run_on_past_its_point() {
    let mut cones = 0;
    for seed in 0..SEEDS {
        // A cylinder's side drawn outward, away from the other (towards it, it may be read for
        // the other when they lie close).
        let task = SolidTask::generate(Solid::Cylinder, seed + 1000);
        let [a, b] = task.answer.sides[0];
        let [c, d] = task.answer.sides[1];
        let long = distance(a, b);
        let mut across = [(a[1] - b[1]) / long * OFF, (b[0] - a[0]) / long * OFF];
        let other = [
            (c[0] + d[0] - a[0] - b[0]) / 2.0,
            (c[1] + d[1] - a[1] - b[1]) / 2.0,
        ];
        if across[0] * other[0] + across[1] * other[1] > 0.0 {
            across = across.map(|v| -v);
        }
        let mut strokes = drawn(&task, (Manner::STEADY, seed), &task.answer.ellipses);
        let side = strokes.len() - 2;
        strokes[side] = [a, b]
            .map(|p| [p[0] + across[0], p[1] + across[1]])
            .to_vec();
        let read = check(&task, &strokes);
        assert!(!read.off.is_empty(), "{seed}");
        for mark in &read.off {
            match mark {
                Off::Ring(p) => assert!(near(*p, &[a, b]), "{seed}: {p:?}"),
                Off::RunOn { line, .. } => {
                    let from = strokes[side].iter().map(|p| distance(line[0], *p));
                    assert!(from.fold(1.0, f64::min) < 1e-6, "{seed}: {line:?}");
                }
                Off::Plumb(_) => {}
            }
        }
        // A cone's longer side from its base aimed beside its point, where its sides are long
        // enough to read apart from its base (a cone seen down its axis is all base).
        let task = SolidTask::generate(Solid::Cone, seed + 1000);
        let point = task.point.expect("a cone's point");
        let sides = &task.answer.sides;
        let length = |k: usize| distance(sides[k][0], sides[k][1]);
        let at = usize::from(length(1) > length(0));
        if length(at) < CONE_SIDE {
            continue;
        }
        cones += 1;
        let [apex, base] = sides[at];
        let long = distance(apex, base);
        let across = [
            (apex[1] - base[1]) / long * OFF,
            (base[0] - apex[0]) / long * OFF,
        ];
        let mut strokes = drawn(&task, (Manner::STEADY, seed), &task.answer.ellipses);
        let side = strokes.len() - 2 + at;
        strokes[side] = vec![base, [apex[0] + across[0], apex[1] + across[1]]];
        let read = check(&task, &strokes);
        let run =
            matches!(read.off[..], [Off::RunOn { past: Some(p), .. }] if distance(p, point) < 1e-9);
        assert!(run, "{seed}: {:?}", read.off);
    }
    println!("cones with a side long enough: {cones} of {SEEDS}");
    assert!(cones * 2 >= SEEDS as usize);
}
