//! The estimates' demonstrations (`demos/demo.rs`, unit 3): a point run across and down against
//! the frame's halves and thirds, a proportion's halves and thirds on both segments, an angle
//! against a level and upright cross and run on to the frame's edges, then marked or drawn and
//! compared as the exercise compares it: the answer red and only what is off (`FlatCheck`'s own
//! marks on the hand's ink, which lands a little off on purpose).

use crate::demos::demo::{
    Demo, DemoLayout, Drawn, Marker, On, Script, clearest, line, off_marks, tick,
};
use crate::demos::demo_points::{THIRD_TICK, crossed, moved, read};
use crate::kinds::estimates::flat::{Estimate, Flat, FlatAnswer, FlatCheck};
use crate::maths::path::{Point, across_frame, distance};

/// The examples' seeds: a point's, a proportion's, an angle's.
const SEEDS: [u64; 3] = [0xDE30_5EED_000A, 0xDE30_5EED_000B, 0xDE30_5EED_000C];
/// A tick's half length across a frame's edge or a segment at a half.
const HALF_TICK: f64 = 0.04;
/// How far a mark estimated by eye lies off the exact place, across and down (frame units).
const OFF: Point = [0.035, -0.025];

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

/// An estimate on the flat: a point, a proportion or an angle.
pub(crate) fn estimate(estimate: Estimate) -> Option<Demo> {
    let seed = match estimate {
        Estimate::Point => SEEDS[0],
        Estimate::Proportion => SEEDS[1],
        Estimate::Angle => SEEDS[2],
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
    let flat = Flat::Point { at };
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
    let check = FlatCheck::of(&flat, &read(&crossed(mark)));
    let mut compared = vec![(On::Copy, Marker::Answer, Drawn::Cross(at))];
    compared.extend(off_marks(&check.off));
    script.compare(DemoLayout::Pair, compared)
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

/// A given segment in `pen` (the original's ink, the copy's printed grey): its line, its first
/// end ringed (the share counts from it), its last barred.
fn barred(script: &mut Script, (on, pen): (On, Marker), [first, last]: [Point; 2]) {
    script.given(on, pen, line(first, last));
    script.given(on, pen, Drawn::Line(tick(last, first, THIRD_TICK)));
    script.given(on, pen, Drawn::Ring(first));
}

fn proportion(original: [Point; 2], share: f64, copy: [Point; 2]) -> Demo {
    let flat = Flat::Proportion {
        original,
        share,
        copy,
    };
    let mut script = Script::new();
    barred(&mut script, (On::Original, Marker::Given), original);
    barred(&mut script, (On::Copy, Marker::Printed), copy);
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
    let drawn = tick(mark, copy[1], HALF_TICK);
    script.draw(On::Copy, Marker::Ink, Drawn::Line(drawn.clone()));
    let check = FlatCheck::of(&flat, &[&drawn]);
    let exact = along(copy, share);
    let mut compared = vec![(
        On::Copy,
        Marker::Answer,
        Drawn::Line(tick(exact, copy[1], HALF_TICK)),
    )];
    compared.extend(off_marks(&check.off));
    script.compare(DemoLayout::Pair, compared)
}

fn angle(flat: &Flat, original: [Point; 2], through: Point) -> Option<Demo> {
    let FlatAnswer::Line(answer) = flat.answer() else {
        return None;
    };
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, line(original[0], original[1]));
    script.given(On::Copy, Marker::Printed, Drawn::Dot(through));
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
    let check = FlatCheck::of(flat, &[mine.as_slice()]);
    let mut compared = vec![(On::Copy, Marker::Answer, line(answer[0], answer[1]))];
    compared.extend(off_marks(&check.off));
    Some(script.compare(DemoLayout::Pair, compared))
}
