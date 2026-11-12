//! The exact answer on 300 generated boxes of each solid: V大's construction, done with lines on
//! the picture alone (a face's diagonals, lines through their crossing towards the vanishing
//! points, a box halved in height), lands on the projected solid; the sides touch the ends; a
//! sphere's outline is its silhouette and V大's circle lies within the Draw pen's width of it.

use openpaint_practice::{
    Ellipse, EllipseGuide, Exercise, Family, Kind, Original, PEN_WIDTH, Point, Pose, Solid,
    SolidTask, diagonals, distance, meet,
};

const SEEDS: u64 = 300;
const CLOSE: f64 = 1e-8;

/// The value of the ellipse's own equation at `p`: 0 on it, negative inside.
fn level(e: &Ellipse, p: Point) -> f64 {
    let (sin, cos) = e.angle.sin_cos();
    let [x, y] = [p[0] - e.centre[0], p[1] - e.centre[1]];
    let (u, v) = (x * cos + y * sin, -x * sin + y * cos);
    (u / e.axes[0]).powi(2) + (v / e.axes[1]).powi(2) - 1.0
}

/// How far the line through `a` and `b` is from touching `e`: its offset against `e`'s reach
/// along its normal, either way.
fn touching(e: &Ellipse, [a, b]: [Point; 2]) -> f64 {
    let long = distance(a, b);
    let n = [(b[1] - a[1]) / long, -(b[0] - a[0]) / long];
    [1.0, -1.0]
        .map(|s| (e.support([s * n[0], s * n[1]]) - s * (n[0] * a[0] + n[1] * a[1])).abs())
        .into_iter()
        .fold(f64::INFINITY, f64::min)
}

fn crossed(a: [Point; 2], b: [Point; 2]) -> Point {
    meet(a, b).expect("the lines meet")
}

/// The line through `through` towards where `a` and `b` meet (their vanishing point), or along
/// them where they never meet (a two-point box's uprights).
fn towards(through: Point, a: [Point; 2], b: [Point; 2]) -> [Point; 2] {
    match meet(a, b) {
        Some(point) if distance(point, through) > 1e-6 => [through, point],
        _ => [
            through,
            [
                through[0] + a[1][0] - a[0][0],
                through[1] + a[1][1] - a[0][1],
            ],
        ],
    }
}

/// V大's middles of `square`, found on the picture: its diagonals' crossing, then lines through
/// it towards where each pair of opposite edges meets, crossing the other pair at their middles.
fn found_middles(square: &EllipseGuide) -> [Point; 4] {
    let c = square.corners;
    let edge = |i: usize| [c[i], c[(i + 1) % 4]];
    let [d0, d1] = diagonals(&c);
    let centre = crossed(d0, d1);
    let along_u = towards(centre, edge(0), edge(2));
    let along_v = towards(centre, edge(1), edge(3));
    [
        crossed(along_v, edge(0)),
        crossed(along_u, edge(1)),
        crossed(along_v, edge(2)),
        crossed(along_u, edge(3)),
    ]
}

fn tasks(solid: Solid) -> impl Iterator<Item = (u64, SolidTask)> {
    (0..SEEDS).map(move |seed| (seed, SolidTask::generate(solid, seed)))
}

#[test]
fn every_solid_keeps_its_rules_and_is_the_same_per_seed() {
    for solid in Solid::ALL {
        let mut poses = [0; 3];
        for (seed, task) in tasks(solid) {
            let label = format!("{solid:?} {seed}");
            assert_eq!(task, SolidTask::generate(solid, seed), "{label}");
            assert_eq!(task.solid, solid);
            for corner in task.seen.corners {
                assert!(corner.iter().all(|v| (0.06..=0.94).contains(v)), "{label}");
            }
            for e in task
                .squares
                .iter()
                .map(|s| s.exact)
                .chain(task.answer.ellipses.clone())
            {
                assert!(e.roundness() >= 0.22, "{label}: {}", e.roundness());
            }
            let sides = if solid == Solid::Sphere { 0 } else { 2 };
            assert_eq!(task.answer.sides.len(), sides, "{label}");
            for [a, b] in &task.answer.sides {
                assert!(distance(*a, *b) >= 0.1, "{label}");
            }
            poses[[Pose::Upright, Pose::Left, Pose::Right]
                .iter()
                .position(|p| *p == task.pose)
                .unwrap_or(0)] += 1;
        }
        if solid != Solid::Sphere {
            assert!(poses.iter().all(|n| *n >= 40), "{solid:?}: {poses:?}");
        }
        let (code, variant) = Kind::Solid(solid).code();
        assert_eq!(code, 13);
        assert_eq!(Kind::of_code(code, variant), Some(Kind::Solid(solid)));
        let family = solid.family();
        assert!(family.facts().compared && !family.facts().aligned);
        let exercise = Exercise::generated(Kind::Solid(solid), 5);
        assert!(
            matches!(exercise.original(), Original::Solid(task) if *task == SolidTask::generate(solid, 5))
        );
    }
    assert_eq!(Kind::of_code(13, 3), None);
    assert_eq!(Kind::of_code(15, 0), None);
    let families = [Family::Cylinder, Family::Cone, Family::Sphere];
    assert_eq!(Solid::ALL.map(Solid::family), families);
}

#[test]
fn v大s_middles_on_the_picture_lie_on_the_exact_ellipses_which_touch_the_edges_there() {
    for solid in Solid::ALL {
        for (seed, task) in tasks(solid) {
            for square in &task.squares {
                let found = found_middles(square);
                for (i, (middle, touch)) in found.iter().zip(square.touches).enumerate() {
                    let label = format!("{solid:?} {seed} middle {i}");
                    assert!(distance(*middle, touch) < CLOSE, "{label}");
                    assert!(level(&square.exact, *middle).abs() < 1e-7, "{label}");
                    let edge = [square.corners[i], square.corners[(i + 1) % 4]];
                    assert!(
                        touching(&square.exact, edge) < 1e-7,
                        "{label}: not touching"
                    );
                }
            }
        }
    }
}

#[test]
fn a_cylinder_s_sides_touch_both_ends_and_a_cone_s_run_from_its_point_touching_its_base() {
    let mut between = 0;
    for (seed, task) in tasks(Solid::Cylinder) {
        let [near, far] = [task.answer.ellipses[0], task.answer.ellipses[1]];
        for side in &task.answer.sides {
            let label = format!("cylinder {seed}");
            assert!(level(&near, side[0]).abs() < 1e-7, "{label}");
            assert!(level(&far, side[1]).abs() < 1e-7, "{label}");
            assert!(touching(&near, *side) < 1e-7, "{label}");
            assert!(touching(&far, *side) < 1e-7, "{label}");
        }
        // The far end hidden behind the cylinder between the sides; the near one seen whole,
        // unless the eye lies between the ends' planes and sees neither face.
        let hidden = |k: usize| task.answer.outlines[k].iter().filter(|s| s.hidden).count();
        assert_eq!(hidden(1), 1, "cylinder {seed}");
        assert!(hidden(0) <= 1, "cylinder {seed}");
        between += usize::from(hidden(0) == 1);
    }
    assert!(between > 0, "some upright cylinder across the horizon");
    for (seed, task) in tasks(Solid::Cone) {
        let label = format!("cone {seed}");
        let top = task.top.expect("the face opposite the base");
        let [d0, d1] = diagonals(&top);
        let point = crossed(d0, d1);
        assert!(
            distance(point, task.point.expect("a point")) < CLOSE,
            "{label}"
        );
        let base = task.answer.ellipses[0];
        for side in &task.answer.sides {
            assert!(distance(side[0], point) < CLOSE, "{label}");
            assert!(level(&base, side[1]).abs() < 1e-7, "{label}");
            assert!(touching(&base, *side) < 1e-7, "{label}");
        }
        assert!(task.answer.outlines[0].iter().filter(|s| s.hidden).count() <= 1);
    }
}

#[test]
fn a_sphere_s_outline_is_its_silhouette_and_v大s_circle_lies_within_the_pen_s_width() {
    let mut worst: f64 = 0.0;
    for (seed, task) in tasks(Solid::Sphere) {
        let label = format!("sphere {seed}");
        let outline = task.answer.ellipses[0];
        let section = task.squares[0];
        // V大's halving on the picture: each side face's diagonals cross at its centre; the line
        // through it towards its vanishing point meets its uprights at their middles.
        let c = task.seen.corners;
        let left = |a: usize, b: usize| [c[a], c[b]];
        let middles = |face: [Point; 4], to: [[Point; 2]; 2]| {
            let [d0, d1] = diagonals(&face);
            let line = towards(crossed(d0, d1), to[0], to[1]);
            (
                crossed(line, [face[0], face[3]]),
                crossed(line, [face[1], face[2]]),
            )
        };
        let (front, back_left) = middles(task.halved[0], [left(0, 4), left(2, 6)]);
        let (front_too, right) = middles(task.halved[1], [left(0, 1), left(2, 3)]);
        let back = crossed(
            towards(right, left(0, 4), left(2, 6)),
            towards(back_left, left(0, 1), left(2, 3)),
        );
        for (found, corner) in [front, right, back, back_left].iter().zip(section.corners) {
            assert!(distance(*found, corner) < CLOSE, "{label}: the section");
        }
        assert!(distance(front, front_too) < CLOSE, "{label}");
        // The middle section is a great circle seen by an eye looking level: its widest points
        // are the outline's, and it lies inside it.
        for way in [[1.0, 0.0], [-1.0, 0.0]] {
            let reach = outline.support(way) - section.exact.support(way);
            assert!(reach.abs() < 1e-9, "{label}: {reach}");
        }
        for p in section.exact.outline(360) {
            assert!(level(&outline, p) < 1e-7, "{label}: outside");
        }
        let off = task.circle_off().expect("a circle");
        assert!(off <= PEN_WIDTH, "{label}: {off}");
        worst = worst.max(off);
    }
    println!(
        "V大's circle off the exact outline at most {:.2} px",
        worst * 400.0
    );
    assert!(SolidTask::generate(Solid::Cylinder, 1).circle().is_none());
}
