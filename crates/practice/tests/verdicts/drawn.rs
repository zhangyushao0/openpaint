//! Drawn kinds judged against their exact answer: an ellipse (its findings), the estimates (a
//! mark's or a line's error against `NEAR_OFF` and `NEAR_ANGLE`) and a point in space (its share
//! of the face against `NEAR_FACE`).

use openpaint_practice::{
    Ellipse, EllipseGuide, Estimate, Exercise, Flat, Guide, Kind, NEAR_ANGLE, NEAR_FACE, NEAR_OFF,
    PEN_WIDTH, Point, REACH, SpaceAnswer, SpaceTask, Trick, Verdict, distance,
};

use crate::{Clean, Near, Redo, checked, moved, steady};

/// A frame's ellipse clearly wider than tall and level, its exercise.
fn level_ellipse() -> (Exercise, EllipseGuide) {
    (0..)
        .map(|seed| Exercise::generated(Kind::Ellipse(Guide::Frame), seed))
        .find_map(|exercise| {
            let guide = *exercise.original().downcast::<EllipseGuide>()?;
            let e = guide.exact;
            ((0.4..0.7).contains(&e.roundness()) && e.angle.abs() < 0.1)
                .then_some((exercise, guide))
        })
        .expect("a wide frame")
}

#[test]
fn an_ellipse_is_clean_near_with_one_finding_redo_with_more_incomplete_unfitted() {
    let (exercise, guide) = level_ellipse();
    let exact = guide.exact;
    let round = |e: Ellipse| steady(vec![e.outline(200)], 1);
    assert_eq!(checked(&exercise, round(exact)), Verdict::App(Clean));
    // Its right side drawn short by two pen widths: one finding.
    let short = Ellipse {
        centre: [exact.centre[0] - REACH, exact.centre[1]],
        axes: [exact.axes[0] - REACH, exact.axes[1]],
        ..exact
    };
    assert_eq!(checked(&exercise, round(short)), Verdict::App(Near));
    // Moved left: short of the right side and past the left.
    let moved = Ellipse {
        centre: [exact.centre[0] - 2.0 * REACH, exact.centre[1]],
        ..exact
    };
    assert_eq!(checked(&exercise, round(moved)), Verdict::App(Redo));
    // A quarter of it: no ellipse to fit yet.
    let quarter: Vec<Point> = exact.outline(200).into_iter().take(50).collect();
    assert_eq!(checked(&exercise, vec![quarter]), Verdict::Incomplete);
}

/// `exercise`'s answer as a hand draws it, moved by `by`.
fn answer_moved(exercise: &Exercise, by: Point) -> Vec<Vec<Point>> {
    let answer = exercise.original().answer();
    let strokes = answer.iter().map(|stroke| moved(stroke, by)).collect();
    steady(strokes, 7)
}

#[test]
fn a_point_or_a_proportion_is_near_within_three_pen_widths_and_redo_past() {
    for estimate in [Estimate::Point, Estimate::Proportion] {
        let exercise = Exercise::generated(Kind::Flat(estimate), 11);
        let Some(&flat) = exercise.original().downcast::<Flat>() else {
            panic!("an estimate")
        };
        // Along the copy's segment for a proportion, across for a point.
        let way = match flat {
            Flat::Proportion { copy, .. } => {
                let long = distance(copy[0], copy[1]);
                [
                    (copy[1][0] - copy[0][0]) / long,
                    (copy[1][1] - copy[0][1]) / long,
                ]
            }
            _ => [1.0, 0.0],
        };
        let off = |by: f64| answer_moved(&exercise, [way[0] * by, way[1] * by]);
        let label = format!("{estimate:?}");
        assert_eq!(checked(&exercise, off(0.0)), Verdict::App(Clean), "{label}");
        assert_eq!(
            checked(&exercise, off(2.0 * PEN_WIDTH)),
            Verdict::App(Near),
            "{label}"
        );
        assert_eq!(
            checked(&exercise, off(NEAR_OFF + 2.0 * PEN_WIDTH)),
            Verdict::App(Redo),
            "{label}"
        );
        // Only a long guide line, no mark yet.
        let guide = vec![vec![[0.1, 0.9], [0.9, 0.9]]];
        assert_eq!(checked(&exercise, guide), Verdict::Incomplete, "{label}");
    }
}

/// `line` turned `degrees` about `through`.
fn turned(line: &[Point], through: Point, degrees: f64) -> Vec<Point> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    (line.iter())
        .map(|p| {
            let [x, y] = [p[0] - through[0], p[1] - through[1]];
            [
                through[0] + cos * x - sin * y,
                through[1] + sin * x + cos * y,
            ]
        })
        .collect()
}

#[test]
fn an_angle_is_near_within_four_degrees_and_redo_past() {
    // A long answer, so a turn past the pen's width at its ends shows.
    let (exercise, through) = (0..)
        .map(|seed| Exercise::generated(Kind::Flat(Estimate::Angle), seed))
        .find_map(|exercise| {
            let Some(&Flat::Angle { through, .. }) = exercise.original().downcast::<Flat>() else {
                return None;
            };
            let line = &exercise.original().answer()[0];
            (distance(line[0], line[1]) > 0.5).then_some((exercise, through))
        })
        .expect("a long angle");
    let line = exercise.original().answer()[0].clone();
    let drawn = |degrees: f64| steady(vec![turned(&line, through, degrees)], 3);
    assert_eq!(checked(&exercise, drawn(0.0)), Verdict::App(Clean));
    assert_eq!(
        checked(&exercise, drawn(NEAR_ANGLE - 0.5)),
        Verdict::App(Near)
    );
    assert_eq!(
        checked(&exercise, drawn(2.0 * NEAR_ANGLE)),
        Verdict::App(Redo)
    );
    // A tick shorter than a line.
    let tick = vec![vec![through, [through[0] + 0.05, through[1]]]];
    assert_eq!(checked(&exercise, tick), Verdict::Incomplete);
}

#[test]
fn a_point_in_space_is_near_within_its_share_of_the_face_and_incomplete_short_of_its_lines() {
    let exercise = Exercise::generated(Kind::Space(Trick::Centre), 5);
    let Some(task) = exercise.original().downcast::<SpaceTask>().cloned() else {
        panic!("a point in space")
    };
    let SpaceAnswer::Point(answer) = task.answer() else {
        panic!("a point")
    };
    let [u, v] = task.on_face(answer);
    let off = |share: f64| {
        let p = task.at([u + share, v]);
        answer_moved(&exercise, [p[0] - answer[0], p[1] - answer[1]])
    };
    assert_eq!(checked(&exercise, off(0.0)), Verdict::App(Clean));
    assert_eq!(checked(&exercise, off(0.9 * NEAR_FACE)), Verdict::App(Near));
    assert_eq!(checked(&exercise, off(3.0 * NEAR_FACE)), Verdict::App(Redo));
    assert_eq!(checked(&exercise, Vec::new()), Verdict::Incomplete);
    // The thirds ask two uprights: one drawn is not yet the copy.
    let thirds = Exercise::generated(Kind::Space(Trick::Thirds), 5);
    let answer = thirds.original().answer();
    assert_eq!(answer.len(), 2);
    assert_eq!(
        checked(&thirds, steady(answer.clone(), 2)),
        Verdict::App(Clean)
    );
    assert_eq!(
        checked(&thirds, steady(answer[..1].to_vec(), 2)),
        Verdict::Incomplete
    );
}
