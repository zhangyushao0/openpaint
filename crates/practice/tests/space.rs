//! Points in space (S90): every answer is a point of the box's face in space projected
//! (`SpaceTask::at`), and V大's five tricks, done with lines on the picture alone (the face's
//! corners, its vanishing point, the given point), land on it exactly: the diagonals' crossing on
//! the centre, the line through it to the vanishing point on the halves, the diagonal across the
//! lines from the near edge's thirds on the thirds, the diagonal through the far edge's middle on
//! the next face, the rectangle between a point and the middle on its mirror. The copy is read
//! back onto the face, its error a share of the face.

use openpaint_practice::{
    Metric, Point, SpaceAnswer, SpaceCheck, SpaceTask, Trick, distance, meet, value,
};

const CLOSE: f64 = 1e-9;
const SEEDS: u64 = 300;

fn lerp(a: Point, b: Point, t: f64) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn crossed(a: [Point; 2], b: [Point; 2]) -> Point {
    meet(a, b).expect("the lines meet")
}

/// The upright through `p`: two-point verticals stay vertical on the picture.
fn upright(p: Point) -> [Point; 2] {
    [p, [p[0], p[1] - 1.0]]
}

fn lines(task: &SpaceTask) -> Vec<[Point; 2]> {
    match task.answer() {
        SpaceAnswer::Lines(lines) => lines,
        SpaceAnswer::Point(_) => panic!("lines"),
    }
}

fn point(task: &SpaceTask) -> Point {
    match task.answer() {
        SpaceAnswer::Point(p) => p,
        SpaceAnswer::Lines(_) => panic!("a point"),
    }
}

#[test]
fn every_task_keeps_its_rules_and_is_the_same_per_seed() {
    for trick in Trick::ALL {
        assert_eq!(SpaceTask::generate(trick, 3), SpaceTask::generate(trick, 3));
        for seed in 0..SEEDS {
            let task = SpaceTask::generate(trick, seed);
            let [near_bottom, far_bottom, far_top, near_top] = task.corners;
            let (near, far) = (
                distance(near_bottom, near_top),
                distance(far_bottom, far_top),
            );
            assert!(near >= 0.2 && far >= 0.1, "{trick:?} {seed}");
            assert!(
                far <= 0.8 * near + 1e-12,
                "{trick:?} {seed}: it recedes plainly"
            );
            assert!(
                distance(near_bottom, far_bottom) >= 0.22,
                "{trick:?} {seed}"
            );
            let answers: Vec<Point> = match task.answer() {
                SpaceAnswer::Point(p) => vec![p],
                SpaceAnswer::Lines(lines) => lines.into_iter().flatten().collect(),
            };
            let inside = |p: &Point| p.iter().all(|v| (0.03..=0.97).contains(v));
            assert!(answers.iter().all(inside), "{trick:?} {seed}: {answers:?}");
            assert!(task.seen.corners.iter().all(inside), "{trick:?} {seed}");
            assert_eq!(task.given.is_some(), trick == Trick::Mirror);
        }
    }
}

#[test]
fn the_face_is_carried_back_from_the_picture_exactly() {
    for seed in 0..SEEDS {
        let task = SpaceTask::generate(Trick::Step, seed);
        for p in [
            [0.0, 0.0],
            [1.0, 1.0],
            [0.5, 0.5],
            [0.2, 0.9],
            [2.0, 0.3],
            [1.7, 1.0],
        ] {
            let back = task.on_face(task.at(p));
            assert!(distance(back, p) < 1e-7, "{seed} {p:?}: {back:?}");
        }
        // The corners drawn are the face's in space, its edges heading for its vanishing point.
        for (corner, p) in task
            .corners
            .iter()
            .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
        {
            assert!(distance(*corner, task.at(p)) < CLOSE);
        }
        let [a, b, c, d] = task.corners;
        assert!(
            distance(crossed([a, b], [d, c]), task.vanishing) < 1e-6,
            "{seed}"
        );
    }
}

#[test]
fn the_diagonals_find_the_centre_and_the_line_to_the_vanishing_point_halves() {
    for seed in 0..SEEDS {
        let task = SpaceTask::generate(Trick::Centre, seed);
        let [a, b, c, d] = task.corners;
        let centre = crossed([a, c], [b, d]);
        assert!(distance(centre, point(&task)) < CLOSE, "{seed}");
        // Not the middle of the drawing: it lies nearer the far edge than the corners' mean.
        let mean = [0, 1].map(|k| (a[k] + b[k] + c[k] + d[k]) / 4.0);
        assert!(task.on_face(mean)[0] < 0.5 - 1e-4, "{seed}");
        let task = SpaceTask::generate(Trick::Halves, seed);
        let [a, b, c, d] = task.corners;
        let through = [crossed([a, c], [b, d]), task.vanishing];
        let line = &lines(&task)[0];
        assert!(
            distance(crossed(through, [a, d]), line[0]) < CLOSE,
            "{seed}"
        );
        assert!(
            distance(crossed(through, [b, c]), line[1]) < CLOSE,
            "{seed}"
        );
    }
}

#[test]
fn the_near_edge_s_thirds_and_a_diagonal_find_the_thirds() {
    for seed in 0..SEEDS {
        let task = SpaceTask::generate(Trick::Thirds, seed);
        let [a, b, c, d] = task.corners;
        let answer = lines(&task);
        for (k, line) in [1.0, 2.0].into_iter().zip(&answer) {
            // An upright edge shows true: its thirds are the picture's.
            let mark = lerp(a, d, k / 3.0);
            let at = crossed([mark, task.vanishing], [a, c]);
            assert!((at[0] - line[0][0]).abs() < CLOSE, "{seed}: x {k}");
            assert!(
                distance(line[0], crossed(upright(at), [a, b])) < CLOSE,
                "{seed}"
            );
            assert!(
                distance(line[1], crossed(upright(at), [d, c])) < CLOSE,
                "{seed}"
            );
        }
        // The near part longer than the far.
        let near = distance(a, answer[0][0]);
        let far = distance(answer[1][0], b);
        assert!(near > far, "{seed}: {near} {far}");
    }
}

#[test]
fn the_diagonal_through_the_far_edge_s_middle_finds_the_next_face() {
    for seed in 0..SEEDS {
        let task = SpaceTask::generate(Trick::Step, seed);
        let [a, b, c, d] = task.corners;
        let middle = crossed([lerp(a, d, 0.5), task.vanishing], [b, c]);
        let corner = crossed([d, middle], [a, task.vanishing]);
        let answer = lines(&task);
        assert!(distance(corner, answer[1][0]) < CLOSE, "{seed}");
        let top = crossed(upright(corner), [d, task.vanishing]);
        assert!(distance(top, answer[1][1]) < CLOSE, "{seed}");
        // Shorter in the picture than the face before it.
        assert!(distance(b, corner) < distance(a, b), "{seed}");
    }
}

#[test]
fn the_rectangle_to_the_middle_finds_the_mirror() {
    for seed in 0..SEEDS {
        let task = SpaceTask::generate(Trick::Mirror, seed);
        let [a, b, c, d] = task.corners;
        let given = task.given_at().expect("a point");
        let middle = upright(crossed([a, c], [b, d]));
        let level = [given, task.vanishing];
        let foot = crossed(upright(given), [a, b]);
        let (top, bottom) = (crossed(level, middle), crossed(middle, [a, b]));
        let x = crossed([foot, top], [bottom, given]);
        let half = crossed([x, task.vanishing], middle);
        let below = crossed([given, half], [a, b]);
        let mirror = crossed(upright(below), level);
        assert!(distance(mirror, point(&task)) < 1e-8, "{seed}");
    }
}

#[test]
fn a_copy_is_read_back_onto_the_face() {
    let off = |check: &SpaceCheck| value(&check.metrics, Metric::SpaceOff);
    for seed in 0..40 {
        let task = SpaceTask::generate(Trick::Centre, seed);
        let exact = point(&task);
        let dot: &[Point] = &[exact, [exact[0] + 1e-4, exact[1]]];
        let read = SpaceCheck::of(&task, &[dot]);
        assert!(off(&read).is_some_and(|v| v < 1e-3), "{seed}: {read:?}");
        let task = SpaceTask::generate(Trick::Thirds, seed);
        let drawn: Vec<Vec<Point>> = lines(&task).iter().map(|l| l.to_vec()).collect();
        let drawn: Vec<&[Point]> = drawn.iter().map(Vec::as_slice).collect();
        assert!(
            off(&SpaceCheck::of(&task, &drawn)).is_some_and(|v| v < 1e-6),
            "{seed}"
        );
        // The picture's equal thirds miss: the near third is longer.
        let [a, b, c, d] = task.corners;
        let even: Vec<[Point; 2]> = [1.0, 2.0]
            .map(|k| [lerp(a, b, k / 3.0), lerp(d, c, k / 3.0)])
            .to_vec();
        let even: Vec<&[Point]> = even.iter().map(|l| l.as_slice()).collect();
        assert!(
            off(&SpaceCheck::of(&task, &even)).is_some_and(|v| v > 0.005),
            "{seed}"
        );
        // A level line through the near edge's middle misses the far one, unless the face's
        // middle lies on the horizon.
        let task = SpaceTask::generate(Trick::Halves, seed);
        let [from, to] = [task.at([0.0, 0.5]), task.at([1.0, 0.5])];
        let level: &[Point] = &[from, [to[0], from[1]]];
        let exact: &[Point] = &lines(&task)[0];
        let read = SpaceCheck::of(&task, &[exact]);
        assert!(off(&read).is_some_and(|v| v < 1e-6), "{seed}");
        let slope = ((to[1] - from[1]) / (to[0] - from[0]))
            .atan()
            .to_degrees()
            .abs();
        if slope > 3.0 {
            assert!(
                off(&SpaceCheck::of(&task, &[level])).is_some_and(|v| v > 1e-3),
                "{seed}"
            );
        }
    }
    let task = SpaceTask::generate(Trick::Step, 5);
    let nothing = SpaceCheck::of(&task, &[]);
    assert!(nothing.metrics.is_empty() && nothing.lines.is_empty());
    let far: &[Point] = &lines(&task)[1];
    assert!(off(&SpaceCheck::of(&task, &[far])).is_some_and(|v| v < 1e-6));
}
