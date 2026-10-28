//! Stage 3's demonstrations (`demo.rs`): the estimates on the flat (a point run across and down
//! against the frame's halves and thirds, a proportion's halves and thirds on both segments, an
//! angle against a level and upright cross and run on to the frame's edges, then marked or drawn)
//! and V大's five tricks for points in space (the trick's construction in order with Sketch, the
//! answer with Draw), each then compared. The tricks' lines are `SpaceTask::construction`'s and
//! their answers `SpaceTask::answer`'s.

use super::{Demo, Drawn, Layout, Marker, On, Script, clearest, line, tick};
use crate::flat::{Estimate, Flat, FlatAnswer};
use crate::path::{Point, across_frame, distance};
use crate::plan::Family;
use crate::space::{SpaceAnswer, SpaceTask, Trick};

/// The examples' seeds, one per kind.
const SEEDS: [u64; 8] = [
    0xDE30_5EED_000A,
    0xDE30_5EED_000B,
    0xDE30_5EED_000C,
    0xDE30_5EED_000D,
    0xDE30_5EED_000E,
    0xDE30_5EED_000F,
    0xDE30_5EED_0010,
    0xDE30_5EED_0011,
];
/// A tick's half length across a frame's edge or a segment, at a half and at a third.
const HALF_TICK: f64 = 0.04;
const THIRD_TICK: f64 = 0.025;
/// How far a mark estimated by eye lies off the exact place, across and down (frame units).
const OFF: Point = [0.035, -0.025];
/// How far a mark or a line found by construction lies off: a hair, as a careful hand draws it.
const NEAR: Point = [0.012, -0.009];

const SHARES: [f64; 3] = [1.0 / 3.0, 0.5, 2.0 / 3.0];

fn tick_of(share: f64) -> f64 {
    if (share - 0.5).abs() < 1e-9 {
        HALF_TICK
    } else {
        THIRD_TICK
    }
}

fn along([a, b]: [Point; 2], share: f64) -> Point {
    [a[0] + (b[0] - a[0]) * share, a[1] + (b[1] - a[1]) * share]
}

fn moved(p: Point, by: Point) -> Point {
    [p[0] + by[0], p[1] + by[1]]
}

/// An estimate on the flat: a point, a proportion or an angle.
pub(super) fn estimate(family: Family) -> Option<Demo> {
    let (estimate, seed) = match family {
        Family::Point => (Estimate::Point, SEEDS[0]),
        Family::Proportion => (Estimate::Proportion, SEEDS[1]),
        Family::Angle => (Estimate::Angle, SEEDS[2]),
        _ => return None,
    };
    // An example the frame's halves and thirds say something of: off the middle, away from the
    // edges, its segments long.
    let flat = clearest(
        (seed, 24),
        |seed| Flat::generate(estimate, seed),
        |flat| match *flat {
            Flat::Point { at } => {
                let inside = at.iter().map(|v| 0.5 - (v - 0.5).abs()).fold(1.0, f64::min);
                let off = at.iter().map(|v| (v - 0.5).abs()).fold(1.0, f64::min);
                inside.min(0.25) + off.min(0.08)
            }
            Flat::Proportion { original, copy, .. } => {
                distance(original[0], original[1]).min(distance(copy[0], copy[1]))
            }
            Flat::Angle { original, .. } => {
                let [dx, dy] = [
                    original[1][0] - original[0][0],
                    original[1][1] - original[0][1],
                ];
                // Neither near level nor near upright, nor at 45 degrees: about 27 from one.
                let turn = dy.atan2(dx).to_degrees().rem_euclid(90.0);
                -(turn - 27.0).abs().min((turn - 63.0).abs())
            }
        },
    );
    Some(match flat {
        Flat::Point { at } => point(at),
        Flat::Proportion {
            original,
            share,
            copy,
        } => proportion(original, share, copy),
        Flat::Angle { original, through } => angle(&flat, original, through)?,
    })
}

/// The frame's halves and thirds ticked on its top and left edges.
fn edge_ticks(on: On) -> Vec<(On, Marker, Drawn)> {
    let mut ticks = Vec::new();
    for share in SHARES {
        let long = tick_of(share);
        ticks.push((on, Marker::Pencil, line([share, 0.0], [share, long])));
        ticks.push((on, Marker::Pencil, line([0.0, share], [long, share])));
    }
    ticks
}

/// The lines across and down a frame through `p`, dashed.
fn runs(on: On, p: Point) -> [(On, Marker, Drawn); 2] {
    [
        (on, Marker::Guide, line([0.0, p[1]], [1.0, p[1]])),
        (on, Marker::Guide, line([p[0], 0.0], [p[0], 1.0])),
    ]
}

fn point(at: Point) -> Demo {
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, Drawn::Dot(at));
    script.step();
    for (on, pen, drawn) in runs(On::Original, at) {
        script.draw(on, pen, drawn);
    }
    script.step();
    script.together(edge_ticks(On::Original));
    script.together(edge_ticks(On::Copy));
    script.step();
    let mark = moved(at, OFF);
    script.draw(On::Copy, Marker::Ink, Drawn::Cross(mark));
    // What was read first, the red over it, as Compare lays them.
    let mut compared = runs(On::Copy, at).to_vec();
    compared.push((On::Copy, Marker::Found, Drawn::Ring(mark)));
    compared.push((On::Copy, Marker::Answer, Drawn::Cross(at)));
    script.compare(Family::Point, Layout::Pair, compared)
}

/// A segment's halves and thirds ticked across it.
fn segment_ticks(on: On, segment: [Point; 2]) -> Vec<(On, Marker, Drawn)> {
    (SHARES.iter())
        .map(|share| {
            let at = along(segment, *share);
            (
                on,
                Marker::Pencil,
                Drawn::Line(tick(at, segment[1], tick_of(*share))),
            )
        })
        .collect()
}

/// A given segment: its line, its first end ringed (the share counts from it), its last barred.
fn barred(script: &mut Script, on: On, [first, last]: [Point; 2]) {
    script.given(on, Marker::Given, line(first, last));
    script.given(
        on,
        Marker::Given,
        Drawn::Line(tick(last, first, THIRD_TICK)),
    );
    script.given(on, Marker::Given, Drawn::Ring(first));
}

fn proportion(original: [Point; 2], share: f64, copy: [Point; 2]) -> Demo {
    let mut script = Script::new();
    barred(&mut script, On::Original, original);
    barred(&mut script, On::Copy, copy);
    script.given(
        On::Original,
        Marker::Given,
        Drawn::Dot(along(original, share)),
    );
    script.step();
    script.together(segment_ticks(On::Original, original));
    script.step();
    script.together(segment_ticks(On::Copy, copy));
    script.step();
    let mark = along(copy, share + 0.04);
    script.draw(
        On::Copy,
        Marker::Ink,
        Drawn::Line(tick(mark, copy[1], HALF_TICK)),
    );
    let exact = along(copy, share);
    let compared = vec![
        (On::Copy, Marker::Found, Drawn::Ring(mark)),
        (
            On::Copy,
            Marker::Answer,
            Drawn::Line(tick(exact, copy[1], HALF_TICK)),
        ),
    ];
    script.compare(Family::Proportion, Layout::Pair, compared)
}

fn angle(flat: &Flat, original: [Point; 2], through: Point) -> Option<Demo> {
    let FlatAnswer::Line(answer) = flat.answer() else {
        return None;
    };
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, line(original[0], original[1]));
    script.given(On::Copy, Marker::Given, Drawn::Dot(through));
    script.step();
    let middle = along(original, 0.5);
    for arm in [[0.1, 0.0], [0.0, 0.1]] {
        let ends = [-1.0, 1.0].map(|s| [middle[0] + s * arm[0], middle[1] + s * arm[1]]);
        script.draw(On::Original, Marker::Guide, line(ends[0], ends[1]));
    }
    script.step();
    let direction = |[a, b]: [Point; 2]| [b[0] - a[0], b[1] - a[1]];
    let run = across_frame(original[0], direction(original))?;
    script.draw(On::Original, Marker::Guide, line(run[0], run[1]));
    script.step();
    // Drawn a few degrees off, through the dot.
    let turn = 3.0_f64.to_radians();
    let (sin, cos) = turn.sin_cos();
    let mine = answer.map(|p| {
        let d = [p[0] - through[0], p[1] - through[1]];
        [
            through[0] + d[0] * cos - d[1] * sin,
            through[1] + d[0] * sin + d[1] * cos,
        ]
    });
    script.draw(On::Copy, Marker::Ink, line(mine[0], mine[1]));
    let read = across_frame(through, direction(mine))?;
    let exact = across_frame(through, direction(answer))?;
    let mut compared = vec![(On::Copy, Marker::Guide, line(exact[0], exact[1]))];
    for arm in [[0.1, 0.0], [0.0, 0.1]] {
        let ends = [-1.0, 1.0].map(|s| [through[0] + s * arm[0], through[1] + s * arm[1]]);
        compared.push((On::Copy, Marker::Guide, line(ends[0], ends[1])));
    }
    compared.push((On::Copy, Marker::Found, line(read[0], read[1])));
    compared.push((On::Copy, Marker::Answer, line(answer[0], answer[1])));
    Some(script.compare(Family::Angle, Layout::Pair, compared))
}

/// A point in space by one of V大's tricks.
pub(super) fn trick(family: Family) -> Option<Demo> {
    // The construction's lines by step; the answer's step draws the last ones listed with it.
    let (trick, seed, steps): (Trick, u64, &[&[usize]]) = match family {
        Family::Centre => (Trick::Centre, SEEDS[3], &[&[0, 1], &[]]),
        Family::Halves => (Trick::Halves, SEEDS[4], &[&[0, 1], &[2], &[]]),
        Family::Thirds => (Trick::Thirds, SEEDS[5], &[&[], &[0, 1], &[2], &[]]),
        Family::Step => (Trick::Step, SEEDS[6], &[&[0], &[2, 3], &[1], &[]]),
        Family::Mirror => (
            Trick::Mirror,
            SEEDS[7],
            &[&[0, 1, 2], &[3, 4], &[5, 6, 7], &[8], &[9]],
        ),
        _ => return None,
    };
    // The face shaded widest across the picture, so its construction reads small.
    let task = clearest(
        (seed, 48),
        |seed| SpaceTask::generate(trick, seed),
        |task| {
            let [near_bottom, far_bottom, far_top, near_top] = task.corners;
            let across = (far_bottom[0] - near_bottom[0])
                .abs()
                .min((far_top[0] - near_top[0]).abs());
            across / distance(near_bottom, near_top).max(0.1)
        },
    );
    let mut script = Script::new();
    given_box(&mut script, &task);
    let construction = task.construction();
    for (index, lines) in steps.iter().enumerate() {
        script.step();
        if trick == Trick::Thirds && index == 0 {
            // The near upright edge shows true: divided in three.
            for v in [1.0 / 3.0, 2.0 / 3.0] {
                let mark = tick(task.at([0.0, v]), task.at([0.0, 0.0]), THIRD_TICK);
                script.draw(On::Copy, Marker::Pencil, Drawn::Line(mark));
            }
        }
        for at in *lines {
            let [a, b] = construction[*at];
            script.draw(On::Copy, Marker::Pencil, line(a, b));
        }
    }
    // The answer with Draw, after the last step's lines.
    let (mine, compared) = answered(&task);
    for drawn in mine {
        script.draw(On::Copy, Marker::Ink, drawn);
    }
    Some(script.compare(family, Layout::Single, compared))
}

/// The box of `task` as the exercise gives it: its face shaded, the horizon, the edges the eye
/// sees, a mirror's dot.
fn given_box(script: &mut Script, task: &SpaceTask) {
    let [near_bottom, far_bottom, far_top, near_top] = task.corners;
    let face = Drawn::Band {
        rows: vec![[near_top, far_top], [near_bottom, far_bottom]],
        colour: None,
    };
    script.given(On::Copy, Marker::Shade, face);
    let height = task.seen.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for ((a, b, _), hidden) in task.seen.edges().into_iter().zip(task.seen.hidden()) {
        if !hidden {
            script.given(On::Copy, Marker::Given, line(a, b));
        }
    }
    if let Some(dot) = task.given_at() {
        script.given(On::Copy, Marker::Given, Drawn::Dot(dot));
    }
}

/// What a hand draws for `task`'s answer (a little off), and Compare's marks: the exact answer
/// red, what was read blue.
fn answered(task: &SpaceTask) -> (Vec<Drawn>, Vec<(On, Marker, Drawn)>) {
    match task.answer() {
        SpaceAnswer::Point(p) => {
            let mark = moved(p, NEAR);
            let compared = vec![
                (On::Copy, Marker::Found, Drawn::Ring(mark)),
                (On::Copy, Marker::Answer, Drawn::Cross(p)),
            ];
            (vec![Drawn::Cross(mark)], compared)
        }
        SpaceAnswer::Lines(lines) => {
            let shift = [NEAR[0], -NEAR[1]];
            let mine: Vec<[Point; 2]> = (lines.iter())
                .map(|[a, b]| [moved(*a, shift), moved(*b, shift)])
                .collect();
            let mut compared: Vec<(On, Marker, Drawn)> = (mine.iter())
                .map(|[a, b]| (On::Copy, Marker::Found, line(*a, *b)))
                .collect();
            compared.extend(
                lines
                    .iter()
                    .map(|[a, b]| (On::Copy, Marker::Answer, line(*a, *b))),
            );
            (mine.iter().map(|[a, b]| line(*a, *b)).collect(), compared)
        }
    }
}
