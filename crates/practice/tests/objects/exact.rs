//! An object's answer and construction (S94): every object keeps its rules; V大's construction,
//! done on the picture from the box alone (`object_construction.rs`), finds every corner of the
//! object where the object in space projects it; a round part's middles are its exact ellipses'.

use openpaint_practice::{
    Carve, Exercise, Kind, LEAST_EDGE, ObjectTask, Point, derived, distance, length,
};

/// Objects of each carve the tests lay.
pub const EACH: u64 = 75;

/// Each carve's `EACH` objects.
pub fn objects() -> impl Iterator<Item = ObjectTask> {
    Carve::ALL.into_iter().flat_map(|carve| {
        (0..EACH).map(move |k| ObjectTask::generate(carve, derived(0x0B1E_C794, k)))
    })
}

/// Whether `p` lies within `reach` of the segment `[a, b]`.
fn on(p: Point, [a, b]: [Point; 2], reach: f64) -> bool {
    let d = [b[0] - a[0], b[1] - a[1]];
    let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
    let t = ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (long * long);
    let near = [
        a[0] + d[0] * t.clamp(0.0, 1.0),
        a[1] + d[1] * t.clamp(0.0, 1.0),
    ];
    distance(p, near) <= reach
}

#[test]
fn every_object_keeps_its_rules() {
    let fractions = [0.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0];
    let (mut two, mut round, mut count) = (0, 0, 0);
    for task in objects() {
        count += 1;
        two += usize::from(task.stages.len() == 2);
        round += usize::from(!task.squares.is_empty());
        assert!(task.seen.vanishing[2].is_none(), "two-point");
        assert!((0.06..=0.94).contains(&task.seen.horizon));
        for [a, b] in &task.answer.sides {
            assert!(
                distance(*a, *b) >= LEAST_EDGE,
                "{:?}: an edge {}",
                task.carve,
                distance(*a, *b)
            );
        }
        let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
        assert!(
            seen.clone()
                .all(|stretch| length(&stretch.points) >= LEAST_EDGE)
        );
        assert!(
            task.squares
                .iter()
                .all(|square| square.exact.roundness() >= 0.22)
        );
        assert!(!task.answer.whole, "only what is seen is drawn");
        for stage in &task.stages {
            assert!(
                !stage.found.is_empty(),
                "{:?}: each change finds corners",
                task.carve
            );
        }
        // Every corner a half, a third or a quarter of the box's edges (but a cone's point, its
        // opposite face's centre).
        let fraction = |v: &f64| fractions.iter().any(|f| (f - v).abs() < 1e-6);
        let off = (task.stages.iter().flat_map(|stage| stage.at.iter()))
            .filter(|p| !p.iter().all(fraction))
            .count();
        assert!(
            off <= usize::from(task.squares.len() == 1),
            "{:?}",
            task.carve
        );
    }
    println!("{count} objects: {two} of two changes, {round} with a round part");
    assert!(two > 0 && round > 0);
}

#[test]
fn an_object_is_the_same_per_seed_and_stored_by_its_carve() {
    for carve in Carve::ALL {
        assert_eq!(
            ObjectTask::generate(carve, 9),
            ObjectTask::generate(carve, 9)
        );
        let (code, variant) = Kind::Object(carve).code();
        assert_eq!(code, 14);
        assert_eq!(Kind::of_code(code, variant), Some(Kind::Object(carve)));
        let exercise = Exercise::generated(Kind::Object(carve), 9);
        assert!(
            exercise.original().downcast::<ObjectTask>() == Some(&ObjectTask::generate(carve, 9))
        );
    }
    let facts = Kind::Object(Carve::Cut).facts();
    assert!(facts.compared && !facts.aligned);
}

#[test]
fn the_construction_on_the_picture_lands_on_every_corner_of_the_object() {
    let (mut found, mut worst) = (0, 0.0_f64);
    for task in objects() {
        let mut lines = task.construction();
        lines.extend(task.seen.edges().iter().map(|(a, b, _)| [*a, *b]));
        let touches: Vec<Point> = task.squares.iter().flat_map(|s| s.touches).collect();
        for stage in &task.stages {
            for (point, at) in stage.found.iter().zip(&stage.at) {
                let off = distance(*point, task.projected(*at));
                worst = worst.max(off);
                assert!(off < 1e-7, "{:?}: {at:?} off by {off}", task.carve);
                // Found where two lines of the construction (or the box) cross.
                let through = lines.iter().filter(|line| on(*point, **line, 1e-9)).count();
                assert!(through >= 2, "{:?}: {at:?} on {through} lines", task.carve);
                found += 1;
            }
            for middle in &stage.middles {
                let near = touches
                    .iter()
                    .map(|t| distance(*t, *middle))
                    .fold(1.0, f64::min);
                assert!(
                    near < 1e-9,
                    "a middle off the exact ellipse's touch by {near}"
                );
            }
        }
        // A round part's middles lie on its exact ellipses (S93's), each touching its square there.
        for square in &task.squares {
            for touch in square.touches {
                assert!(square.exact.distance(touch) < 1e-8);
            }
        }
    }
    println!("{found} corners found, at most {worst:.1e} off the projection");
}
