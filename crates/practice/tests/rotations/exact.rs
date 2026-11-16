//! A rotation's two views (S113): each keeps its scene's rules seed after seed (the picture's
//! every one of unit 4's, the asked view those its answer needs), every change shows in the
//! picture, the asked view is turned as its level says, and V大's construction on the asked box
//! finds every corner of the object where the object in space projects it.

use openpaint_practice::{
    Exercise, Kind, LEAST_EDGE, ObjectTask, PerspectiveBox, Point, Rotation, RotationTask, Turning,
    derived, distance, length,
};

/// Rotations of each variant the tests lay.
pub const EACH: u64 = 40;

/// Each variant's `EACH` rotations.
pub fn rotations() -> impl Iterator<Item = RotationTask> {
    Rotation::ALL.into_iter().flat_map(|rotation| {
        (0..EACH).map(move |k| RotationTask::generate(rotation, derived(0x0113_7A7E, k)))
    })
}

/// Where a box lies against the eye's level: below it (-1), across it (0) or above it (1).
fn side(seen: &PerspectiveBox) -> i8 {
    let below = seen.corners.iter().all(|c| c[1] > seen.horizon);
    let above = seen.corners.iter().all(|c| c[1] < seen.horizon);
    match (below, above) {
        (true, _) => -1,
        (_, true) => 1,
        _ => 0,
    }
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

/// The rules an object's view keeps that its answer is drawn by: a two-point box in the frame,
/// its horizon too, every visible edge and seen curve `LEAST_EDGE` long, its ellipses no slivers,
/// each change finding corners.
fn drawn_by_its_rules(task: &ObjectTask, view: &str) {
    let inside = |v: f64| (0.06..=0.94).contains(&v);
    assert!(task.seen.vanishing[2].is_none(), "{view}: two-point");
    assert!(inside(task.seen.horizon), "{view}: {}", task.seen.horizon);
    assert!(
        (task.seen.corners.iter()).all(|c| inside(c[0]) && inside(c[1])),
        "{view}: the box in the frame"
    );
    let angle = task.seen.front_angle();
    assert!(angle >= 90.0, "{view}: a front corner of {angle:.1}°");
    for [a, b] in &task.answer.sides {
        assert!(distance(*a, *b) >= LEAST_EDGE, "{view}: an edge");
    }
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    assert!(
        seen.clone().all(|s| length(&s.points) >= LEAST_EDGE),
        "{view}"
    );
    assert!(
        task.squares.iter().all(|s| s.exact.roundness() >= 0.22),
        "{view}"
    );
    assert!(!task.answer.whole && !task.stages.is_empty(), "{view}");
    assert!(
        task.stages.iter().all(|stage| !stage.found.is_empty()),
        "{view}"
    );
}

#[test]
fn both_views_keep_their_rules_and_the_asked_one_is_turned_as_its_level_says() {
    let (mut count, mut two, mut round) = (0, 0, 0);
    for task in rotations() {
        count += 1;
        let (shown, asked) = (&task.shown, &task.asked);
        let named = |view: &str| format!("{:?} {}: {view}", task.rotation, task.quarters);
        drawn_by_its_rules(shown, &named("the picture's view"));
        drawn_by_its_rules(asked, &named("the asked view"));
        // One design: the same carve, its changes, its round parts.
        assert_eq!(shown.carve, task.rotation.carve);
        assert_eq!(asked.carve, task.rotation.carve);
        assert_eq!(shown.stages.len(), asked.stages.len(), "the same changes");
        assert_eq!(shown.squares.is_empty(), asked.squares.is_empty());
        two += usize::from(shown.stages.len() == 2);
        let posted = !shown.squares.is_empty();
        round += usize::from(posted);
        let sides = [side(&shown.seen), side(&asked.seen)];
        match task.rotation.turning {
            Turning::Quarter => {
                assert!(matches!(task.quarters, 1 | 3), "a quarter either way");
                assert_eq!(shown.stages.len(), 1, "one change");
                assert_eq!(sides[0], sides[1], "the eye's height kept");
                assert!(task.angle.is_none());
            }
            Turning::Height | Turning::Bare => {
                assert!((1..=3).contains(&task.quarters));
                // A post is always seen from above, from another band of heights.
                assert!(posted || sides[0] != sides[1], "another height: {sides:?}");
                assert!(!posted || sides == [-1, -1]);
            }
        }
        if task.rotation.turning == Turning::Bare {
            assert!(posted, "a round part standing on its base");
            let angle = task.angle.expect("the small angle box");
            assert_eq!(angle.vanishing, asked.seen.vanishing, "at the asked angle");
            let low =
                |b: &PerspectiveBox, k: usize| b.corners.iter().map(|c| c[k]).fold(9.0, f64::min);
            let high =
                |b: &PerspectiveBox, k: usize| b.corners.iter().map(|c| c[k]).fold(-9.0, f64::max);
            let apart = (0..2).any(|k| {
                low(&angle, k) > high(&asked.seen, k) || high(&angle, k) < low(&asked.seen, k)
            });
            assert!(apart, "clear of the box");
            assert!((0..2).all(|k| low(&angle, k) >= 0.0 && high(&angle, k) <= 1.0));
        }
    }
    println!("{count} rotations: {two} of two changes, {round} with a round part");
    assert!(round > 0);
}

#[test]
fn every_change_shows_in_the_picture() {
    for task in rotations() {
        for (k, stage) in task.shown.stages.iter().enumerate() {
            let shown = stage.at.iter().filter(|p| task.shown.shows(**p)).count();
            assert!(shown >= 2, "{:?}: change {k} shows {shown}", task.rotation);
        }
        // The asked view shows a change at least: never the bare box.
        let asked = &task.asked;
        let any = (asked.stages.iter())
            .any(|stage| stage.at.iter().filter(|p| asked.shows(**p)).count() >= 2);
        assert!(any, "{:?}", task.rotation);
    }
}

#[test]
fn the_asked_view_s_corners_lie_where_its_construction_finds_them() {
    let (mut found, mut worst) = (0, 0.0_f64);
    for task in rotations() {
        let asked = &task.asked;
        let mut lines = asked.construction();
        lines.extend(asked.seen.edges().iter().map(|(a, b, _)| [*a, *b]));
        let touches: Vec<Point> = asked.squares.iter().flat_map(|s| s.touches).collect();
        for stage in &asked.stages {
            for (point, at) in stage.found.iter().zip(&stage.at) {
                let off = distance(*point, asked.projected(*at));
                worst = worst.max(off);
                assert!(off < 1e-7, "{:?}: {at:?} off by {off}", task.rotation);
                let through = lines.iter().filter(|line| on(*point, **line, 1e-9)).count();
                assert!(
                    through >= 2,
                    "{:?}: {at:?} on {through} lines",
                    task.rotation
                );
                found += 1;
            }
            for middle in &stage.middles {
                let near = (touches.iter())
                    .map(|t| distance(*t, *middle))
                    .fold(1.0, f64::min);
                assert!(near < 1e-9, "a middle off its exact ellipse by {near}");
            }
        }
    }
    println!("{found} corners found, at most {worst:.1e} off the projection");
}

#[test]
fn a_rotation_is_the_same_per_seed_and_stored_by_its_variant() {
    for (place, rotation) in Rotation::ALL.into_iter().enumerate() {
        let task = RotationTask::generate(rotation, 9);
        assert_eq!(task, RotationTask::generate(rotation, 9));
        let kind = Kind::Rotation(rotation);
        assert_eq!(kind.code(), (22, place as u8));
        assert_eq!(Kind::of_code(22, place as u8), Some(kind));
        let exercise = Exercise::generated(kind, 9);
        assert!(exercise.original().downcast::<RotationTask>() == Some(&task));
    }
    assert_eq!(Kind::of_code(22, 5), None);
    let facts = Kind::Rotation(Rotation::ALL[0]).facts();
    assert!(facts.compared && !facts.aligned && !facts.constructed);
}
