//! Recognition on hand-like fixtures: dense strokes with a hand's wobble, fast strokes with the
//! author's sample spacing (16 to 37 px between samples, the turns cut), and the samples piling up
//! while the pen holds still at the end. Lines, arcs, circles and ellipses here; polylines, what
//! stays as drawn, the samples along a shape and its handles in `quickshape_polyline_tests.rs`.

use super::*;
use crate::quickshape::Shape;

/// The hold's radius at fit on the tablet, in canvas pixels (5 points).
const STILL: f32 = 10.0;

/// A smooth wobble of `amount` px, different for each `seed`.
fn wobble(points: &[Point], amount: f32, seed: f32) -> Vec<Point> {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let t = i as f32;
            [
                p[0] + amount * (t * 0.23 + seed).sin(),
                p[1] + amount * (t * 0.31 + 2.0 * seed).cos(),
            ]
        })
        .collect()
}

/// The path through `corners` taken every `step` px.
fn polyline(corners: &[Point], step: f32) -> Vec<Point> {
    let mut out = Vec::new();
    for pair in corners.windows(2) {
        let n = (norm(sub(pair[1], pair[0])) / step).ceil().max(1.0) as usize;
        out.extend((0..n).map(|k| {
            let t = k as f32 / n as f32;
            [
                pair[0][0] + (pair[1][0] - pair[0][0]) * t,
                pair[0][1] + (pair[1][1] - pair[0][1]) * t,
            ]
        }));
    }
    out.extend(corners.last());
    out
}

/// An ellipse's path from angle `from` turning `sweep` (degrees), `n` samples.
fn oval(
    centre: Point,
    [a, b]: [f32; 2],
    tilt: f32,
    (from, sweep): (f32, f32),
    n: usize,
) -> Vec<Point> {
    let (sin, cos) = tilt.to_radians().sin_cos();
    (0..n)
        .map(|k| {
            let t = (from + sweep * k as f32 / (n - 1) as f32).to_radians();
            let (u, v) = (a * t.cos(), b * t.sin());
            [centre[0] + u * cos - v * sin, centre[1] + u * sin + v * cos]
        })
        .collect()
}

/// `points` with the pen resting at the end: 40 samples within half a pixel of it.
fn held(mut points: Vec<Point>) -> Vec<Point> {
    let end = *points.last().expect("end");
    points.extend((0..40).map(|k| {
        [
            end[0] + 0.4 * (k as f32).sin(),
            end[1] + 0.3 * (k as f32).cos(),
        ]
    }));
    points
}

fn near(a: Point, b: Point, within: f32) -> bool {
    norm(sub(a, b)) <= within
}

#[test]
fn lines_dense_fast_and_slightly_bent_snap_to_their_ends() {
    let dense = held(wobble(
        &polyline(&[[100.0, 100.0], [700.0, 260.0]], 3.0),
        3.0,
        0.4,
    ));
    let fast = held(wobble(
        &polyline(&[[100.0, 100.0], [700.0, 260.0]], 31.0),
        2.0,
        1.1,
    ));
    let bent: Vec<Point> = (0..=150)
        .map(|k| {
            let t = k as f32 / 150.0;
            [100.0 + 600.0 * t, 300.0 + 40.0 * 4.0 * t * (1.0 - t) * 0.3]
        })
        .collect();
    for path in [dense, fast, held(bent)] {
        match recognise(&path, STILL) {
            Some(Shape::Line([from, to])) => {
                assert_eq!(from, path[0]);
                assert_eq!(to, *path.last().expect("end"));
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn an_arc_snaps_to_its_circle_from_end_to_end() {
    for (step, seed) in [(3.0, 0.2), (28.0, 0.9)] {
        let n = (300.0 * 120f32.to_radians() / step) as usize;
        let path = held(wobble(
            &oval([400.0, 400.0], [300.0, 300.0], 0.0, (200.0, 120.0), n),
            3.0,
            seed,
        ));
        let Some(Shape::Arc(points)) = recognise(&path, STILL) else {
            panic!("{:?}", recognise(&path, STILL));
        };
        let (centre, radius, _, sweep) = crate::quickshape::circle_through(points).expect("arc");
        assert!(near(centre, [400.0, 400.0], 9.0), "{centre:?}");
        assert!((radius - 300.0).abs() < 9.0, "{radius}");
        assert!(
            (sweep.to_degrees() - 120.0).abs() < 4.0,
            "{}",
            sweep.to_degrees()
        );
        assert!(near(points[0], path[0], 8.0));
    }
}

#[test]
fn circles_dense_and_fast_snap_round_even_drawn_past_the_start() {
    let dense = wobble(
        &oval([500.0, 400.0], [200.0, 200.0], 0.0, (30.0, 372.0), 420),
        4.0,
        0.7,
    );
    // 30 samples 31 px apart, the last 20 degrees short of the start.
    let fast = wobble(
        &oval([500.0, 400.0], [150.0, 145.0], 0.0, (-60.0, 340.0), 30),
        2.0,
        2.0,
    );
    for (path, radius) in [(held(dense), 200.0), (held(fast), 147.5)] {
        let Some(Shape::Ellipse {
            centre,
            axes,
            sense,
            ..
        }) = recognise(&path, STILL)
        else {
            panic!("{:?}", recognise(&path, STILL));
        };
        let [u, v] = axes.map(norm);
        assert_eq!(u, v, "round");
        assert!((u - radius).abs() < radius * 0.03, "{u} against {radius}");
        assert!(near(centre, [500.0, 400.0], 6.0), "{centre:?}");
        assert_eq!(sense, 1.0, "drawn turning from x towards y");
    }
}

#[test]
fn a_tilted_ellipse_keeps_its_axes_angle_and_start() {
    let path = held(wobble(
        &oval([600.0, 500.0], [300.0, 120.0], 30.0, (90.0, -360.0), 360),
        3.0,
        0.3,
    ));
    let Some(Shape::Ellipse {
        centre,
        axes: [u, v],
        start,
        sense,
    }) = recognise(&path, STILL)
    else {
        panic!("{:?}", recognise(&path, STILL));
    };
    assert!(near(centre, [600.0, 500.0], 6.0), "{centre:?}");
    assert!(
        (norm(u) - 300.0).abs() < 9.0 && (norm(v) - 120.0).abs() < 6.0,
        "{u:?} {v:?}"
    );
    let angle = u[1].atan2(u[0]).to_degrees().rem_euclid(180.0);
    assert!((angle - 30.0).abs() < 2.0, "{angle}");
    assert_eq!(sense, -1.0);
    let begins = [
        centre[0] + u[0] * start.cos() + v[0] * start.sin(),
        centre[1] + u[1] * start.cos() + v[1] * start.sin(),
    ];
    assert!(near(begins, path[0], 8.0), "{begins:?} {:?}", path[0]);
}

#[test]
fn the_held_end_does_not_weigh_in() {
    let path = wobble(
        &oval([400.0, 300.0], [180.0, 180.0], 0.0, (0.0, 360.0), 200),
        3.0,
        0.8,
    );
    let mut piled = path.clone();
    let end = *path.last().expect("end");
    piled.extend((0..500).map(|_| end));
    assert_eq!(recognise(&path, STILL), recognise(&piled, STILL));
}

#[path = "quickshape_polyline_tests.rs"]
mod polylines;
