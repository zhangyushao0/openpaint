//! Organic arrows' exactness (S156l): the ribbon's edges lie at its width either side of its path
//! within 1e-7, as the frame shows them; each edge sample is seen or hidden as the ribbon's depth
//! says (found again another way: the ray against each ruling of a finer ribbon); each half turn
//! folds the ribbon once, where its edges cross; the scene rules hold seed after seed.

use super::ribbon::{Arrow, SAMPLES};
use super::task::ArrowTask;
use crate::maths::path::{Point, crossings, distance};
use crate::maths::random::derived;
use crate::maths::vector::{Vector, add, cross, dot, norm, sub, times, unit};

/// Arrows of each variant the tests lay.
const EACH: u64 = 16;
/// Within this a point lies where it should (world and frame units).
const EXACT: f64 = 1e-7;

fn seeded() -> impl Iterator<Item = (u64, ArrowTask)> {
    Arrow::ALL.into_iter().flat_map(|arrow| {
        (0..EACH).map(move |k| {
            let seed = derived(0x0156_7E57, k);
            (seed, ArrowTask::generate(arrow, seed))
        })
    })
}

fn tasks() -> impl Iterator<Item = ArrowTask> {
    seeded().map(|(_, task)| task)
}

#[test]
fn the_edges_lie_at_the_ribbon_s_width_either_side_of_its_path_within_1e_7() {
    for task in tasks() {
        let ribbon = &task.ribbon;
        let half = ribbon.half;
        for (i, sample) in ribbon.samples.iter().enumerate() {
            let [first, second] = [half, -half].map(|u| ribbon.across(i, u));
            let width = sub(first, second);
            assert!(
                (norm(width) - 2.0 * half).abs() < EXACT,
                "{:?} {i}",
                task.arrow
            );
            assert!(
                dot(width, sample.tangent).abs() < EXACT,
                "square to the path"
            );
            let middle = times(add(first, second), 0.5);
            assert!(norm(sub(middle, sample.at)) < EXACT, "about the path");
            for (side, p) in [first, second].into_iter().enumerate() {
                let shown = task.answer.edges[side][i];
                assert!(distance(shown, task.view.frame(p)) < EXACT);
            }
            assert!(distance(task.path[i], task.view.frame(sample.at)) < EXACT);
        }
        // The near width is where the edges leave the head's base, the far where they end.
        let [tip, a, b] = task.head;
        for p in task.near {
            let along = [b[0] - a[0], b[1] - a[1]];
            let off = (along[0] * (p[1] - a[1]) - along[1] * (p[0] - a[0])) / norm_2(along);
            assert!(off.abs() < 1e-9, "on the head's base");
        }
        assert!(distance(tip, task.path[0]) > distance(task.near[0], task.near[1]) / 2.0);
        assert_eq!(
            task.far,
            [
                task.answer.edges[0][SAMPLES - 1],
                task.answer.edges[1][SAMPLES - 1]
            ]
        );
    }
}

fn norm_2(p: Point) -> f64 {
    p[0].hypot(p[1])
}

/// Whether the ray from the eye to `p` is crossed short of it by a ruling of the ribbon `fine`
/// (points and unit normals at fine samples, `half` either side), but those of `own`: `Some(true)`
/// hidden, `Some(false)` seen, none too near to call (a ruling's end, `p`'s own depth, or a ray
/// grazing a fold, which meets two rulings close together).
fn hidden_by_rulings(
    p: Vector,
    fine: &[(Vector, Vector)],
    half: f64,
    own: std::ops::Range<usize>,
) -> Option<bool> {
    let side = |(c, n): (Vector, Vector)| dot(p, cross(c, n));
    let roots: Vec<usize> = (0..fine.len() - 1)
        .filter(|k| !own.contains(k))
        .filter(|&k| (side(fine[k]) >= 0.0) != (side(fine[k + 1]) >= 0.0))
        .collect();
    let mut doubtful = false;
    for (r, &k) in roots.iter().enumerate() {
        let (a, b) = (side(fine[k]), side(fine[k + 1]));
        let grazing = (roots.get(r + 1).is_some_and(|next| next - k <= GRAZING))
            || (r > 0 && k - roots[r - 1] <= GRAZING);
        let share = a / (a - b);
        let lerp = |x: Vector, y: Vector| add(times(x, 1.0 - share), times(y, share));
        let c = lerp(fine[k].0, fine[k + 1].0);
        let n = unit(lerp(fine[k].1, fine[k + 1].1));
        let (pn, cn) = (cross(p, n), cross(c, n));
        let far = dot(cn, pn) / dot(pn, pn);
        let u = dot(sub(times(p, far), c), n);
        let within = u.abs() < half * 0.99;
        let outside = u.abs() > half * 1.01;
        if far > 0.0 && far < 1.0 && !outside && grazing {
            return None;
        }
        if far > 0.0 && far < 0.99 && within {
            return Some(true);
        }
        if far > 0.0 && far < 1.0 && !outside {
            doubtful = true;
        }
    }
    (!doubtful).then_some(false)
}

/// Two rulings a ray meets this close (fine samples) mean it grazes a fold.
const GRAZING: usize = 8;

#[test]
fn each_edge_sample_is_seen_or_hidden_as_the_ribbon_s_depth_says() {
    let (mut hidden, mut decided) = (0, 0);
    for task in tasks().step_by(3) {
        let ribbon = &task.ribbon;
        let fine: Vec<(Vector, Vector)> =
            (ribbon
                .path
                .samples(4 * (SAMPLES - 1) + 1, [0.0, 1.0, 0.0], &ribbon.turns))
            .iter()
            .map(|s| (s.at, s.normal))
            .collect();
        let head = task.head;
        for side in 0..2 {
            let u = if side == 0 { ribbon.half } else { -ribbon.half };
            for i in 0..SAMPLES {
                let p = ribbon.across(i, u);
                // Its own segments either side, as the answer leaves them out.
                let own = (4 * i).saturating_sub(8)..4 * i + 8;
                let shown = task.answer.edges[side][i];
                let by_head = in_triangle(shown, head);
                let Some(found) = hidden_by_rulings(p, &fine, ribbon.half, own) else {
                    continue;
                };
                if by_head {
                    continue;
                }
                decided += 1;
                hidden += usize::from(found);
                assert_eq!(
                    task.answer.seen[side][i], !found,
                    "{:?} side {side} sample {i}",
                    task.arrow
                );
            }
        }
    }
    println!("{decided} decided, {hidden} hidden");
    assert!(hidden > 200 && decided > 10 * hidden / 2);
}

/// Whether `p` lies in the triangle `[a, b, c]` or near it.
fn in_triangle(p: Point, [a, b, c]: [Point; 3]) -> bool {
    let turn = |x: Point, y: Point| (y[0] - x[0]) * (p[1] - x[1]) - (y[1] - x[1]) * (p[0] - x[0]);
    let [s, t, r] = [turn(a, b), turn(b, c), turn(c, a)];
    let near = [a, b, c].iter().any(|v| distance(*v, p) < 0.02);
    near || (s >= 0.0 && t >= 0.0 && r >= 0.0) || (s <= 0.0 && t <= 0.0 && r <= 0.0)
}

#[test]
fn each_half_turn_folds_the_ribbon_once_where_its_edges_cross() {
    let mut turned = 0;
    for task in tasks() {
        let ribbon = &task.ribbon;
        let edges = &task.answer.edges;
        // Above and below: the ribbon stands, its first edge over its second, but where it turns.
        let over = |i: usize| edges[0][i][1] < edges[1][i][1];
        let flips: Vec<usize> = (1..SAMPLES).filter(|&i| over(i) != over(i - 1)).collect();
        assert_eq!(flips.len(), ribbon.turns.len(), "{:?}", task.arrow);
        assert!(flips.iter().all(|&i| ribbon.turning(i)), "{:?}", task.arrow);
        let folds = task.turned_folds();
        assert_eq!(folds.len(), ribbon.turns.len(), "{:?}", task.arrow);
        for turn in &ribbon.turns {
            let span = |s: f64| (s * (SAMPLES - 1) as f64).round() as usize;
            let [from, to] = [turn.at - turn.span, turn.at + turn.span].map(span);
            let crossed = crossings(&edges[0][from..=to], &edges[1][from..=to]);
            assert_eq!(crossed.len(), 1, "{:?}: the edges cross once", task.arrow);
            let fold = (folds.iter())
                .flat_map(|k| task.answer.folds[*k].iter().flat_map(|s| &s.points))
                .map(|p| distance(*p, crossed[0]))
                .fold(f64::INFINITY, f64::min);
            assert!(
                fold < 0.02,
                "{:?}: the fold at the crossing, {fold}",
                task.arrow
            );
            turned += 1;
        }
    }
    assert!(turned >= EACH as usize);
}

#[test]
fn the_scene_rules_hold_seed_after_seed() {
    for (seed, task) in seeded() {
        let named = format!("{:?} {seed}", task.arrow);
        assert!(task.keeps(), "{named}");
        assert_eq!(task, ArrowTask::generate(task.arrow, seed), "{named}");
        let inside = |p: &Point| p.iter().all(|v| (0.05..=0.95).contains(v));
        assert!(task.answer.edges.iter().flatten().all(inside), "{named}");
        assert!(
            task.head
                .iter()
                .chain(task.ticks.iter().flatten())
                .all(inside)
        );
        let near = distance(task.near[0], task.near[1]);
        let far = distance(task.far[0], task.far[1]);
        assert!(far <= near / 2.0 && far >= 0.016, "{named}: {near} {far}");
        let bends: Vec<Point> = task.ribbon.bends.iter().map(|&i| task.path[i]).collect();
        assert!(bends.len() >= 3, "{named}");
        assert!(
            bends.windows(2).all(|w| distance(w[0], w[1]) >= 0.04),
            "{named}"
        );
        // Each bend folds the ribbon once, each half turn once.
        let folds = task.answer.folds.len();
        assert_eq!(folds, bends.len() + task.ribbon.turns.len(), "{named}");
        match task.arrow {
            Arrow::Gentle => {
                assert!(task.behind_far().is_empty(), "{named}");
                assert_eq!(task.ticks.len(), 4, "{named}: near and far ticked");
            }
            Arrow::Crossing | Arrow::Turning => {
                assert!(!task.crossings().is_empty(), "{named}");
                assert_eq!(task.ticks.len(), 2, "{named}: the near alone ticked");
            }
        }
        let turns = task.ribbon.turns.len();
        let asked = if task.arrow == Arrow::Turning {
            1..=2
        } else {
            0..=0
        };
        assert!(asked.contains(&turns), "{named}");
    }
}
