//! Only what is off (S98) on the estimates and the points in space: simulated hands (S84's steady
//! and shaky manners) drawing the exact answer get no mark; a mark more than the Draw pen's width
//! off has the answer ringed, one within it nothing; an angle turned from the answer is run on
//! across its frame, one beside the dot rings it; a halving line that misses the vanishing point is
//! run on past it, one through the point but off the centre rings the centre; an upright off its
//! third or the next face's rings the point V大's construction finds it at, a leaning one gets its
//! true vertical. Drawn copies judged by eye are never read (S145, D123).

#[expect(
    dead_code,
    reason = "S84's hand: the answers are drawn with its lines alone"
)]
#[path = "boxes/hand.rs"]
mod hand;

use hand::{Hand, Manner};
use openpaint_practice::{
    Estimate, Flat, FlatAnswer, FlatCheck, Off, PEN_WIDTH, Point, SpaceAnswer, SpaceCheck,
    SpaceTask, Trick, distance,
};

const SEEDS: u64 = 200;

/// A small cross at `p`, two strokes.
fn cross(p: Point) -> [Vec<Point>; 2] {
    [
        vec![[p[0] - 0.02, p[1]], [p[0] + 0.02, p[1]]],
        vec![[p[0], p[1] - 0.02], [p[0], p[1] + 0.02]],
    ]
}

fn lines(strokes: &[Vec<Point>]) -> Vec<&[Point]> {
    strokes.iter().map(Vec::as_slice).collect()
}

fn moved(p: Point, by: f64, turn: f64) -> Point {
    [p[0] + by * turn.cos(), p[1] + by * turn.sin()]
}

fn rings(off: &[Off]) -> Vec<Point> {
    (off.iter())
        .filter_map(|mark| match mark {
            Off::Ring(p) => Some(*p),
            _ => None,
        })
        .collect()
}

#[test]
fn a_mark_is_ringed_only_beyond_a_pen_s_width_off() {
    for seed in 0..SEEDS {
        let flat = Flat::generate(Estimate::Point, seed);
        let FlatAnswer::Point(at) = flat.answer() else {
            panic!("a point")
        };
        let centre = SpaceTask::generate(Trick::Centre, seed);
        let SpaceAnswer::Point(middle) = centre.answer() else {
            panic!("a centre")
        };
        for k in 0..8 {
            let turn = f64::from(k) * std::f64::consts::FRAC_PI_4;
            for (by, marked) in [(0.9, false), (1.2, true)] {
                let near = FlatCheck::of(&flat, &lines(&cross(moved(at, by * PEN_WIDTH, turn))));
                assert_eq!(rings(&near.off), if marked { vec![at] } else { vec![] });
                let mark = moved(middle, by * PEN_WIDTH, turn);
                let read = SpaceCheck::of(&centre, &lines(&cross(mark)));
                assert_eq!(read.off.len(), usize::from(marked), "{seed} {k} {by}");
            }
        }
    }
    // A proportion by its error along the copy's segment.
    let flat = Flat::Proportion {
        original: [[0.1, 0.2], [0.9, 0.2]],
        share: 0.25,
        copy: [[0.2, 0.6], [0.8, 0.6]],
    };
    for (along, marked) in [(0.9, false), (1.2, true)] {
        let x = 0.35 + along * PEN_WIDTH;
        let tick: &[Point] = &[[x, 0.57], [x, 0.63]];
        let read = FlatCheck::of(&flat, &[tick]);
        let ringed = rings(&read.off);
        assert_eq!(ringed.len(), usize::from(marked), "{along}");
        assert!(ringed.iter().all(|p| distance(*p, [0.35, 0.6]) < 1e-12));
    }
}

#[test]
fn an_angle_turned_away_is_run_on_across_its_frame_and_one_beside_the_dot_rings_it() {
    for seed in 0..SEEDS {
        let flat = Flat::generate(Estimate::Angle, seed);
        let (Flat::Angle { through, .. }, FlatAnswer::Line([a, b])) = (flat, flat.answer()) else {
            panic!("an angle")
        };
        for manner in [Manner::STEADY, Manner::SHAKY] {
            let drawn = Hand::new(seed, manner).line(a, b);
            let read = FlatCheck::of(&flat, &[drawn.as_slice()]);
            assert!(read.off.is_empty(), "{seed}: {:?}", read.off);
        }
        // Turned 4 degrees about the dot: run on to the frame's edges, nothing ringed.
        let (sin, cos) = 4.0_f64.to_radians().sin_cos();
        let turned = [a, b].map(|p| {
            let d = [p[0] - through[0], p[1] - through[1]];
            [
                through[0] + d[0] * cos - d[1] * sin,
                through[1] + d[0] * sin + d[1] * cos,
            ]
        });
        let read = FlatCheck::of(&flat, &[turned.as_slice()]);
        let [Off::RunOn { line, past: None }] = read.off[..] else {
            panic!("{seed}: run on: {:?}", read.off)
        };
        let on_edge = |p: Point| p.iter().any(|v| v.abs() < 1e-9 || (v - 1.0).abs() < 1e-9);
        assert!(on_edge(line[0]) && on_edge(line[1]), "{line:?}");
        // Beside the dot by two pen widths, at the angle: the dot ringed.
        let long = distance(a, b);
        let across = [(a[1] - b[1]) / long, (b[0] - a[0]) / long];
        let by = 2.0 * PEN_WIDTH;
        let beside = [a, b].map(|p| [p[0] + by * across[0], p[1] + by * across[1]]);
        let read = FlatCheck::of(&flat, &[beside.as_slice()]);
        assert_eq!(read.off, vec![Off::Ring(through)], "{seed}");
    }
}

/// `task`'s answer lines drawn by a hand in `manner` (the next face's far upright alone).
fn answered(task: &SpaceTask, manner: Manner, seed: u64) -> Vec<Vec<Point>> {
    let SpaceAnswer::Lines(answer) = task.answer() else {
        panic!("lines")
    };
    let wanted = if task.trick == Trick::Step {
        &answer[1..2]
    } else {
        &answer[..]
    };
    let mut hand = Hand::new(seed, manner);
    wanted.iter().map(|[a, b]| hand.line(*a, *b)).collect()
}

#[test]
fn a_trick_s_lines_drawn_on_the_answer_get_nothing() {
    for trick in [Trick::Halves, Trick::Thirds, Trick::Step] {
        for manner in [Manner::STEADY, Manner::SHAKY] {
            for seed in 0..SEEDS {
                let task = SpaceTask::generate(trick, seed);
                let read = SpaceCheck::of(&task, &lines(&answered(&task, manner, seed)));
                assert!(read.off.is_empty(), "{trick:?} {seed}: {:?}", read.off);
            }
        }
    }
}

#[test]
fn a_trick_s_lines_are_marked_where_they_miss() {
    for seed in 0..SEEDS {
        // Halves drawn level: run on past the vanishing point, ringed (unless level lies within a
        // pen's width of it: nothing).
        let task = SpaceTask::generate(Trick::Halves, seed);
        let at = |u: f64, v: f64| task.at([u, v]);
        let start = at(0.0, 0.5);
        let level = [start, [at(1.0, 0.5)[0], start[1]]];
        let read = SpaceCheck::of(&task, &[level.as_slice()]);
        let missed =
            matches!(read.off[..], [Off::RunOn { past: Some(p), .. }] if p == task.vanishing);
        let within = distance(at(1.0, 0.5), level[1]) <= PEN_WIDTH && read.off.is_empty();
        assert!(missed || within, "{seed}: {:?}", read.off);
        // Through the vanishing point but well above the middle: the centre ringed.
        let high = [at(0.0, 0.65), at(1.0, 0.65)];
        let read = SpaceCheck::of(&task, &[high.as_slice()]);
        assert_eq!(read.off, vec![Off::Ring(at(0.5, 0.5))], "{seed}");
        // The far third's upright beside its place (away from the near one, which an upright
        // drawn between them is read for when nearer): the point on the diagonal ringed.
        let task = SpaceTask::generate(Trick::Thirds, seed);
        let at = |u: f64, v: f64| task.at([u, v]);
        let shift = 2.5 * PEN_WIDTH;
        let away = (at(2.0 / 3.0, 0.0)[0] - at(1.0 / 3.0, 0.0)[0]).signum() * shift;
        let far = [at(2.0 / 3.0, 0.0), at(2.0 / 3.0, 1.0)].map(|p| [p[0] + away, p[1]]);
        let near = [at(1.0 / 3.0, 0.0), at(1.0 / 3.0, 1.0)];
        let read = SpaceCheck::of(&task, &[near.as_slice(), far.as_slice()]);
        let third = at(2.0 / 3.0, 2.0 / 3.0);
        assert_eq!(read.off, vec![Off::Ring(third)], "{seed}");
        // Leaning 5 degrees: its true vertical.
        let [foot, top] = [at(2.0 / 3.0, 0.0), at(2.0 / 3.0, 1.0)];
        let lean = (top[1] - foot[1]).abs() * 5.0_f64.to_radians().tan();
        let leaning = [foot, [top[0] + lean, top[1]]];
        let read = SpaceCheck::of(&task, &[near.as_slice(), leaning.as_slice()]);
        let plumb = matches!(read.off[..], [Off::Plumb(_)]);
        assert!(plumb, "{seed}: {:?}", read.off);
        // The next face's upright beside its place: its foot ringed.
        let task = SpaceTask::generate(Trick::Step, seed);
        let at = |u: f64, v: f64| task.at([u, v]);
        let upright = [at(2.0, 0.0), at(2.0, 1.0)].map(|p| [p[0] - shift, p[1]]);
        let read = SpaceCheck::of(&task, &[upright.as_slice()]);
        assert_eq!(read.off, vec![Off::Ring(at(2.0, 0.0))], "{seed}");
    }
}
