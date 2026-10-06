//! Recognition on hand-like fixtures: dense strokes with a hand's wobble, fast strokes with the
//! author's sample spacing (16 to 37 px between samples, the turns cut), and the samples piling up
//! while the pen holds still at the end.

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

/// A rectangle 500 x 300 at (200, 150), its corners rounded by the hand (radius 15), begun in
/// the middle of its top side.
fn rounded_rectangle(step: f32) -> Vec<Point> {
    let (x0, y0, x1, y1, r) = (200.0, 150.0, 700.0, 450.0, 15.0);
    let mut path = vec![[450.0, y0]];
    let corners = [
        ([x1 - r, y0 + r], -90.0),
        ([x1 - r, y1 - r], 0.0),
        ([x0 + r, y1 - r], 90.0),
        ([x0 + r, y0 + r], 180.0),
    ];
    for (centre, from) in corners {
        path.extend(oval(centre, [r, r], 0.0, (from, 90.0), 6));
    }
    path.push([450.0, y0]);
    polyline(&path, step)
}

#[test]
fn rectangles_dense_and_fast_snap_to_their_four_corners() {
    let truth = [
        [200.0, 150.0],
        [700.0, 150.0],
        [700.0, 450.0],
        [200.0, 450.0],
    ];
    for (path, within) in [
        (held(wobble(&rounded_rectangle(3.0), 2.0, 0.5)), 6.0),
        (held(wobble(&rounded_rectangle(33.0), 1.5, 1.7)), 12.0),
    ] {
        let Some(Shape::Polyline {
            corners,
            closed: true,
        }) = recognise(&path, STILL)
        else {
            panic!("{:?}", recognise(&path, STILL));
        };
        assert_eq!(corners.len(), 4, "{corners:?}");
        for corner in truth {
            assert!(
                corners.iter().any(|c| near(*c, corner, within)),
                "{corner:?} in {corners:?}"
            );
        }
    }
}

#[test]
fn a_triangle_keeps_its_three_corners() {
    let triangle = held(wobble(
        &polyline(
            &[
                [300.0, 100.0],
                [550.0, 500.0],
                [80.0, 480.0],
                [295.0, 110.0],
            ],
            4.0,
        ),
        2.0,
        0.1,
    ));
    let Some(Shape::Polyline {
        corners,
        closed: true,
    }) = recognise(&triangle, STILL)
    else {
        panic!("{:?}", recognise(&triangle, STILL));
    };
    assert_eq!(corners.len(), 3, "{corners:?}");
    for corner in [[300.0, 100.0], [550.0, 500.0], [80.0, 480.0]] {
        assert!(
            corners.iter().any(|c| near(*c, corner, 8.0)),
            "{corner:?} in {corners:?}"
        );
    }
}

#[test]
fn open_polylines_keep_their_corners_and_ends() {
    let vee = held(wobble(
        &polyline(&[[100.0, 100.0], [300.0, 500.0], [520.0, 120.0]], 20.0),
        2.0,
        0.6,
    ));
    let zigzag = held(wobble(
        &polyline(
            &[
                [100.0, 400.0],
                [250.0, 150.0],
                [400.0, 400.0],
                [550.0, 150.0],
                [700.0, 400.0],
            ],
            4.0,
        ),
        2.0,
        1.4,
    ));
    for (path, count) in [(vee, 3), (zigzag, 5)] {
        let Some(Shape::Polyline {
            corners,
            closed: false,
        }) = recognise(&path, STILL)
        else {
            panic!("{:?}", recognise(&path, STILL));
        };
        assert_eq!(corners.len(), count, "{corners:?}");
        assert!(
            near(corners[0], path[0], 6.0)
                && near(
                    *corners.last().expect("end"),
                    *path.last().expect("end"),
                    6.0
                )
        );
    }
}

#[test]
fn scribbles_spirals_waves_and_short_strokes_stay_as_drawn() {
    let spiral: Vec<Point> = (0..400)
        .map(|k| {
            let t = k as f32 * 0.04;
            [
                500.0 + (20.0 + 12.0 * t) * t.cos(),
                400.0 + (20.0 + 12.0 * t) * t.sin(),
            ]
        })
        .collect();
    let wave: Vec<Point> = (0..300)
        .map(|k| {
            [
                100.0 + 2.0 * k as f32,
                300.0 + 60.0 * (k as f32 * 0.04).sin(),
            ]
        })
        .collect();
    let scribble: Vec<Point> = (0..300)
        .map(|k| {
            let t = k as f32 * 0.05;
            [
                400.0 + 150.0 * (t * 1.3).sin() + 60.0 * (t * 3.7).cos(),
                300.0 + 120.0 * (t * 2.1).cos(),
            ]
        })
        .collect();
    let short = polyline(&[[100.0, 100.0], [140.0, 110.0]], 2.0);
    for (name, path) in [
        ("spiral", spiral),
        ("wave", wave),
        ("scribble", scribble),
        ("short", short),
    ] {
        assert_eq!(recognise(&held(path), STILL), None, "{name}");
    }
}

fn drawn_samples(points: &[Point]) -> Vec<openpaint_core::StrokeSample> {
    let n = points.len();
    points
        .iter()
        .enumerate()
        .map(|(i, [x, y])| {
            let position = openpaint_core::CanvasPoint::new(*x, *y).expect("point");
            let share = i as f32 / (n - 1) as f32;
            openpaint_core::StrokeSample::new(position, 0.2 + 0.6 * share, 0.1, 0.0, 4 * i as u32)
                .expect("sample")
        })
        .collect()
}

#[test]
fn samples_lie_on_the_exact_shape_dense_at_corners_with_the_drawn_pressure() {
    let drawn = drawn_samples(&polyline(&[[0.0, 0.0], [300.0, 0.0]], 3.0));
    let circle = Shape::Ellipse {
        centre: [500.0, 400.0],
        axes: [[150.0, 0.0], [0.0, 150.0]],
        start: 0.3,
        sense: 1.0,
    };
    let outline = circle.outline();
    assert!(
        outline
            .iter()
            .all(|p| (norm(sub(*p, [500.0, 400.0])) - 150.0).abs() < 1e-3)
    );
    assert_eq!(outline.first(), outline.last(), "closed exactly");
    let steps: Vec<f32> = outline.windows(2).map(|w| norm(sub(w[1], w[0]))).collect();
    assert!(
        steps.iter().all(|s| *s <= 13.2),
        "5 degrees at most: {:?}",
        steps.iter().copied().fold(0.0, f32::max)
    );
    let samples = crate::quickshape::along(&outline, &drawn).expect("along");
    let half = samples.len() / 2;
    assert!((samples[0].pressure() - 0.2).abs() < 1e-4);
    assert!(
        (samples[half].pressure() - 0.5).abs() < 0.02,
        "half way: {}",
        samples[half].pressure()
    );
    assert!((samples[samples.len() - 1].pressure() - 0.8).abs() < 1e-4);
    assert!(samples.windows(2).all(|w| w[0].time_ms() <= w[1].time_ms()));
    let square = Shape::Polyline {
        corners: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
        closed: true,
    };
    let outline = square.outline();
    for corner in [[100.0, 0.0], [100.0, 100.0], [0.0, 100.0]] {
        let at = outline
            .iter()
            .position(|p| *p == corner)
            .expect("the corner itself");
        assert!(
            norm(sub(outline[at - 1], corner)) <= 0.5 + 1e-4,
            "a sample just before"
        );
        assert!(
            norm(sub(outline[at + 1], corner)) <= 0.5 + 1e-4,
            "and just after"
        );
    }
    assert!(
        outline
            .windows(2)
            .all(|w| norm(sub(w[1], w[0])) <= 16.0 + 1e-3)
    );
}

#[test]
fn handles_move_ends_corners_and_axes_and_the_held_pen_turns_the_shape() {
    let line = Shape::Line([[0.0, 0.0], [100.0, 0.0]]);
    assert_eq!(
        line.moved(1, [50.0, 50.0]),
        Shape::Line([[0.0, 0.0], [50.0, 50.0]])
    );
    let dragged = line.dragged([100.0, 0.0], [0.0, 200.0]);
    let Shape::Line([from, to]) = dragged else {
        panic!()
    };
    assert_eq!(from, [0.0, 0.0]);
    assert!(near(to, [0.0, 200.0], 1e-3), "{to:?}");
    let ellipse = Shape::Ellipse {
        centre: [0.0, 0.0],
        axes: [[100.0, 0.0], [0.0, 50.0]],
        start: 0.0,
        sense: 1.0,
    };
    let Shape::Ellipse { axes: [u, v], .. } = ellipse.moved(0, [0.0, 80.0]) else {
        panic!()
    };
    assert!(
        near(u, [0.0, 80.0], 1e-4) && near(v, [-50.0, 0.0], 1e-4),
        "{u:?} {v:?}"
    );
    let Shape::Ellipse { axes: [u, v], .. } = ellipse.moved(3, [0.0, 70.0]) else {
        panic!()
    };
    assert!(
        near(u, [-100.0, 0.0], 1e-4) && near(v, [0.0, -70.0], 1e-4),
        "{u:?} {v:?}"
    );
    let Shape::Ellipse {
        axes: [u, _],
        centre,
        ..
    } = ellipse.dragged([100.0, 0.0], [0.0, 150.0])
    else {
        panic!()
    };
    assert_eq!(centre, [0.0, 0.0]);
    assert!(
        near(u, [0.0, 150.0], 1e-3),
        "turned a quarter and grown by half: {u:?}"
    );
    let arc = Shape::Arc([[100.0, 0.0], [0.0, 100.0], [-100.0, 0.0]]);
    assert_eq!(arc.handles().len(), 3);
    let outline = arc.outline();
    assert!(
        outline
            .iter()
            .all(|p| (norm(*p) - 100.0).abs() < 1e-3 && p[1] >= -1e-3),
        "the upper half"
    );
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
