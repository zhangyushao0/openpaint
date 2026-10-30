//! Ellipses (S84): the guides and their exact ellipses (touching every side at its middle as the
//! eye sees it), the direct least-squares fit on known ellipses (exact, trembling, drawn through
//! twice, overshooting, a partial arc; a line holds none), and the words: rounder or flatter,
//! tilted which way, which side missed.

use openpaint_practice::{
    Ellipse, EllipseCheck, EllipseGuide, Exercise, Guide, Kind, Original, Point, Random, Reach,
    Roundness, Side, Tilt, distance, fit_ellipse,
};

/// The value of the ellipse's own equation at `p`: 0 on it, negative inside.
fn level(e: &Ellipse, p: Point) -> f64 {
    let (sin, cos) = e.angle.sin_cos();
    let [x, y] = [p[0] - e.centre[0], p[1] - e.centre[1]];
    let (u, v) = (x * cos + y * sin, -x * sin + y * cos);
    (u / e.axes[0]).powi(2) + (v / e.axes[1]).powi(2) - 1.0
}

fn close(a: &Ellipse, b: &Ellipse, centre: f64, axes: f64, degrees: f64) -> bool {
    let turned = (a.angle - b.angle).to_degrees().rem_euclid(180.0);
    let turned = turned.min(180.0 - turned);
    distance(a.centre, b.centre) <= centre
        && (0..2).all(|k| (a.axes[k] - b.axes[k]).abs() <= axes * b.axes[k])
        && (turned <= degrees || b.roundness() > 0.97)
}

#[test]
fn every_guide_s_exact_ellipse_touches_each_side_at_its_middle() {
    for guide in Guide::ALL {
        for seed in 0..500 {
            let label = format!("{guide:?} {seed}");
            let g = EllipseGuide::generate(guide, seed);
            assert_eq!(g, EllipseGuide::generate(guide, seed), "{label}");
            let roundness = g.exact.roundness();
            assert!((0.26..=0.97).contains(&roundness), "{label}: {roundness}");
            for c in g.corners {
                assert!(
                    c.iter().all(|v| (0.06..=0.94).contains(v)),
                    "{label}: {c:?}"
                );
            }
            let middle = [0, 1].map(|k| g.corners.iter().map(|p| p[k]).sum::<f64>() / 4.0);
            for (i, touch) in g.touches.iter().enumerate() {
                let (a, b) = (g.corners[i], g.corners[(i + 1) % 4]);
                let d = [b[0] - a[0], b[1] - a[1]];
                let long = d[0].hypot(d[1]);
                let mut normal = [d[1] / long, -d[0] / long];
                if normal[0] * (a[0] - middle[0]) + normal[1] * (a[1] - middle[1]) < 0.0 {
                    normal = [-normal[0], -normal[1]];
                }
                let side = normal[0] * a[0] + normal[1] * a[1];
                let on_side = normal[0] * touch[0] + normal[1] * touch[1];
                assert!((on_side - side).abs() < 1e-9, "{label}: off its side");
                assert!(
                    level(&g.exact, *touch).abs() < 1e-7,
                    "{label}: off the ellipse"
                );
                // Tangent there: the ellipse reaches the side and no farther.
                let reach = g.exact.support(normal);
                assert!(
                    (reach - side).abs() < 1e-7,
                    "{label}: {reach} against {side}"
                );
            }
        }
    }
}

#[test]
fn a_plain_frame_s_ellipse_is_its_cell_s_and_a_plane_s_is_seen_in_perspective() {
    let g = EllipseGuide::generate(Guide::Frame, 3);
    let [a, _, c, _] = g.corners;
    let middle = [(a[0] + c[0]) / 2.0, (a[1] + c[1]) / 2.0];
    assert!(distance(g.exact.centre, middle) < 1e-9);
    let sides = [(c[0] - a[0]) / 2.0, (c[1] - a[1]) / 2.0];
    let axes = [sides[0].max(sides[1]), sides[0].min(sides[1])];
    assert!((g.exact.axes[0] - axes[0]).abs() < 1e-9 && (g.exact.axes[1] - axes[1]).abs() < 1e-9);
    // In perspective the touches are no longer the sides' middles: the nearer half looks larger.
    let shifted = (0..100).filter(|seed| {
        let g = EllipseGuide::generate(Guide::Plane, *seed);
        (0..4).any(|i| {
            let (p, q) = (g.corners[i], g.corners[(i + 1) % 4]);
            distance(g.touches[i], [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]) > 0.01
        })
    });
    assert!(shifted.count() > 80);
    let exercise = Exercise::generated(Kind::Ellipse(Guide::Plane), 9);
    assert!(
        matches!(exercise.original(), Original::Ellipse(e) if e == EllipseGuide::generate(Guide::Plane, 9))
    );
    assert_eq!(Kind::of_code(10, 1), Some(Kind::Ellipse(Guide::Plane)));
}

/// `e` drawn as a hand draws it: `loops` times round from `start` (radians), each point off by
/// up to `tremor`, `extra` of a turn more.
fn drawn(
    e: &Ellipse,
    (loops, start, extra): (f64, f64, f64),
    tremor: f64,
    seed: u64,
) -> Vec<Point> {
    let mut random = Random::new(seed);
    let steps = (240.0 * (loops + extra)) as usize;
    (0..=steps)
        .map(|k| {
            let t = start + k as f64 / 240.0 * std::f64::consts::TAU;
            let p = e.at(t);
            [
                p[0] + random.range(-tremor, tremor),
                p[1] + random.range(-tremor, tremor),
            ]
        })
        .collect()
}

#[test]
fn the_fit_finds_known_ellipses_drawn_by_hand() {
    let known = Ellipse {
        centre: [0.48, 0.53],
        axes: [0.3, 0.14],
        angle: 0.4,
    };
    // Read along its outline's chords: within their sagitta.
    let exact = fit_ellipse(&[&known.outline(200)]).expect("a fit");
    assert!(close(&exact, &known, 1e-5, 1e-4, 1e-3), "{exact:?}");
    for seed in 0..40 {
        // Trembling, drawn through twice and past the start.
        let line = drawn(&known, (2.0, 0.3, 0.15), 0.004, seed);
        let fit = fit_ellipse(&[&line]).expect("a fit");
        assert!(close(&fit, &known, 0.005, 0.03, 1.5), "{seed}: {fit:?}");
        // Two thirds of it only.
        let arc = drawn(&known, (0.66, 1.0, 0.0), 0.002, seed);
        let fit = fit_ellipse(&[&arc]).expect("a fit");
        assert!(close(&fit, &known, 0.015, 0.08, 4.0), "{seed}: {fit:?}");
    }
    // In two strokes; a circle; a line holds none; too few points none.
    let halves = [
        drawn(&known, (0.55, 0.0, 0.0), 0.0, 1),
        drawn(&known, (0.55, 3.2, 0.0), 0.0, 2),
    ];
    let fit = fit_ellipse(&[&halves[0], &halves[1]]).expect("a fit");
    assert!(close(&fit, &known, 1e-4, 1e-3, 0.05), "{fit:?}");
    let circle = Ellipse {
        centre: [0.5, 0.5],
        axes: [0.2, 0.2],
        angle: 0.0,
    };
    let fit = fit_ellipse(&[&circle.outline(100)]).expect("a fit");
    assert!(close(&fit, &circle, 1e-5, 1e-3, 90.0), "{fit:?}");
    assert_eq!(fit_ellipse(&[&[[0.1, 0.1], [0.9, 0.5]]]), None);
    assert_eq!(fit_ellipse(&[&[[0.5, 0.5], [0.51, 0.5]]]), None);
}

#[test]
fn a_points_distance_from_an_ellipse_is_the_least_to_its_outline() {
    let mut random = Random::new(93);
    for _ in 0..2000 {
        let e = Ellipse {
            centre: [random.range(0.3, 0.7), random.range(0.3, 0.7)],
            axes: [random.range(0.1, 0.3), 0.0],
            angle: random.range(-1.5, 1.5),
        };
        let e = Ellipse {
            axes: [e.axes[0], e.axes[0] * random.range(0.2, 1.0)],
            ..e
        };
        let on = e.at(random.range(0.0, std::f64::consts::TAU));
        let p = [
            on[0] + random.range(-0.12, 0.12),
            on[1] + random.range(-0.12, 0.12),
        ];
        let dense = e.outline(20_000);
        let least = (dense.windows(2))
            .map(|w| {
                let d = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
                let t = (((p[0] - w[0][0]) * d[0] + (p[1] - w[0][1]) * d[1])
                    / (d[0] * d[0] + d[1] * d[1]))
                    .clamp(0.0, 1.0);
                distance(p, [w[0][0] + t * d[0], w[0][1] + t * d[1]])
            })
            .fold(f64::INFINITY, f64::min);
        let found = e.distance(p);
        assert!((found - least).abs() < 1e-6, "{e:?} {p:?}: {found} {least}");
    }
}

#[test]
fn the_words_say_rounder_or_flatter_tilted_which_way_and_the_side_missed() {
    // A frame clearly wider than tall, so the minor axis grown stays the minor one.
    let guide = (0..)
        .map(|seed| EllipseGuide::generate(Guide::Frame, seed))
        .find(|g| (0.4..0.7).contains(&g.exact.roundness()) && g.exact.angle.abs() < 0.1)
        .expect("a wide frame");
    let exact = guide.exact;
    let words = |e: Ellipse| EllipseCheck::of(&guide, &[&e.outline(200)]);
    let same = words(exact);
    assert!(same.fitted.is_some());
    assert_eq!((same.roundness, same.tilt), (None, None));
    assert!(same.sides.is_empty(), "{:?}", same.sides);
    let rounder = words(Ellipse {
        axes: [exact.axes[0], exact.axes[1] * 1.25],
        ..exact
    });
    assert_eq!(rounder.roundness, Some(Roundness::Rounder));
    let flatter = words(Ellipse {
        axes: [exact.axes[0], exact.axes[1] * 0.75],
        ..exact
    });
    assert_eq!(flatter.roundness, Some(Roundness::Flatter));
    let clockwise = words(Ellipse {
        angle: exact.angle + 8f64.to_radians(),
        ..exact
    });
    assert_eq!(clockwise.tilt, Some(Tilt::Clockwise));
    let anticlockwise = words(Ellipse {
        angle: exact.angle - 8f64.to_radians(),
        ..exact
    });
    assert_eq!(anticlockwise.tilt, Some(Tilt::Anticlockwise));
    // Moved left: past the left side, short of the right; nothing said of top and bottom.
    let moved = words(Ellipse {
        centre: [exact.centre[0] - 0.04, exact.centre[1]],
        ..exact
    });
    let sides: Vec<(Side, Reach)> = moved.sides.iter().map(|m| (m.side, m.reach)).collect();
    assert_eq!(
        sides,
        vec![(Side::Right, Reach::Short), (Side::Left, Reach::Past)]
    );
    // Each ringed at its middle, where the exact one touches it.
    assert_eq!(moved.sides[0].middle, guide.touches[1]);
    assert_eq!(moved.sides[1].middle, guide.touches[3]);
    // A pen's width off is no miss; a little more is.
    let shift = |by: f64| {
        let e = Ellipse {
            centre: [exact.centre[0] - by, exact.centre[1]],
            ..exact
        };
        words(e).sides.len()
    };
    assert_eq!(shift(0.9 * openpaint_practice::REACH), 0);
    assert_eq!(shift(1.2 * openpaint_practice::REACH), 2);
    let none = EllipseCheck::of(&guide, &[&[[0.2, 0.2], [0.8, 0.8]]]);
    assert_eq!(none.fitted, None);
    assert!(none.sides.is_empty() && none.roundness.is_none());
}
