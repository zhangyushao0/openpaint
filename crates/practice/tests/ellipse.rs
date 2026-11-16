//! Ellipses (S84): the guides and their exact ellipses (touching every side at its middle as the
//! eye sees it), which Compare lays over the copy for the learner's eye (S145), and a point's
//! distance from an ellipse.

use openpaint_practice::{Ellipse, EllipseGuide, Exercise, Guide, Kind, Point, Random, distance};

/// The value of the ellipse's own equation at `p`: 0 on it, negative inside.
fn level(e: &Ellipse, p: Point) -> f64 {
    let (sin, cos) = e.angle.sin_cos();
    let [x, y] = [p[0] - e.centre[0], p[1] - e.centre[1]];
    let (u, v) = (x * cos + y * sin, -x * sin + y * cos);
    (u / e.axes[0]).powi(2) + (v / e.axes[1]).powi(2) - 1.0
}

#[test]
fn every_guide_s_exact_ellipse_touches_each_side_at_its_middle() {
    // A funnel lays three ellipses, no one guide (`funnels.rs`).
    for guide in Guide::ALL.into_iter().filter(|g| *g != Guide::Funnel) {
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
        exercise.original().downcast::<EllipseGuide>()
            == Some(&EllipseGuide::generate(Guide::Plane, 9))
    );
    assert_eq!(Kind::of_code(10, 1), Some(Kind::Ellipse(Guide::Plane)));
}

/// S142: a turned frame is a plain frame turned 15 to 75 degrees either way, its ellipse its
/// cell's, its axes along its sides; turned both ways.
#[test]
fn a_turned_frame_is_a_plain_frame_turned_well_off_upright() {
    let (mut clockwise, mut anticlockwise) = (0, 0);
    for seed in 0..500 {
        let label = format!("Turned {seed}");
        let g = EllipseGuide::generate(Guide::Turned, seed);
        let side = |i: usize| {
            let (a, b) = (g.corners[i], g.corners[(i + 1) % 4]);
            [b[0] - a[0], b[1] - a[1]]
        };
        // A rectangle: each corner square.
        for i in 0..4 {
            let [u, v] = [side(i), side((i + 1) % 4)];
            let cos = (u[0] * v[0] + u[1] * v[1]) / (u[0].hypot(u[1]) * v[0].hypot(v[1]));
            assert!(cos.abs() < 1e-9, "{label}: corner {i}");
        }
        let [x, y] = side(0);
        let turn = y.atan2(x).to_degrees();
        let off_upright = turn.rem_euclid(90.0);
        assert!((15.0..=75.0).contains(&off_upright), "{label}: {turn}");
        if turn.rem_euclid(180.0) < 90.0 {
            clockwise += 1;
        } else {
            anticlockwise += 1;
        }
        let [a, _, c, _] = g.corners;
        let middle = [(a[0] + c[0]) / 2.0, (a[1] + c[1]) / 2.0];
        assert!(distance(g.exact.centre, middle) < 1e-9, "{label}");
        let axis = g.exact.angle.to_degrees().rem_euclid(90.0);
        assert!(
            (axis - off_upright).abs() < 1e-6 || g.exact.roundness() > 0.97,
            "{label}"
        );
    }
    assert!(
        clockwise > 150 && anticlockwise > 150,
        "{clockwise} {anticlockwise}"
    );
    assert_eq!(Kind::of_code(10, 2), Some(Kind::Ellipse(Guide::Turned)));
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
