//! Contours' exactness (S156k): the scene rules hold seed after seed; each section's circle lies on
//! the tube and its ellipse runs through the circle's image within 1e-7; its stretches are seen
//! and hidden as the depth test says; its ticks lie on its ellipse where the traced outline runs;
//! the picture is the tube lit on paper.

use super::sausage::SEGMENTS;
use super::sections::Contour;
use super::task::{Bend, ContourTask};
use crate::geometry::ellipse::Ellipse;
use crate::maths::path::{Point, distance, nearest};
use crate::maths::random::derived;
use crate::maths::vector::{dot, norm, sub};

/// Tasks of each variant the tests lay.
const EACH: u64 = 12;
/// Within this (frame units) a point lies on its ellipse; (world units) on the tube.
const EXACT: f64 = 1e-7;
const ON: f64 = 1e-9;

fn tasks() -> impl Iterator<Item = ContourTask> {
    Bend::ALL.into_iter().flat_map(|bend| {
        (0..EACH).map(move |k| ContourTask::generate(bend, derived(0x0156_7E57, k)))
    })
}

/// Every contour of `task`: the given one, then the asked ones.
fn every(task: &ContourTask) -> impl Iterator<Item = &Contour> {
    task.example.iter().chain(&task.contours)
}

#[test]
fn the_scene_rules_hold_seed_after_seed() {
    for task in tasks() {
        let (bend, sausage) = (task.bend, &task.sausage);
        let r = sausage.radius;
        let named = format!("{bend:?} {:?}", task.extent);
        assert!(sausage.tightest() >= 2.2 * r, "{named}: bends too tight");
        assert!(
            sausage.closest_apart(3.0 * r) >= 2.2 * r,
            "{named}: meets itself"
        );
        let across: Vec<&Contour> = every(&task).filter(|c| !c.cap).collect();
        for contour in &across {
            let exact = contour.guide.exact;
            assert!((0.12..=0.9).contains(&exact.roundness()), "{named}");
            assert!(2.0 * exact.axes[0] >= 0.06, "{named}");
            assert!(!contour.ticks.is_empty(), "{named}: ticked");
        }
        for pair in across.windows(2) {
            let [a, b] = [pair[0].guide.exact, pair[1].guide.exact];
            assert!(
                distance(a.centre, b.centre) >= a.axes[0] + b.axes[0],
                "{named}"
            );
        }
        let inside = |v: f64| (0.04..=0.96).contains(&v);
        assert!(
            task.extent.iter().all(|p| inside(p[0]) && inside(p[1])),
            "{named}"
        );
        let caps = task.contours.iter().filter(|c| c.cap).count();
        let asked = task.contours.len() - caps;
        match bend {
            Bend::Gentle => {
                assert_eq!((asked, caps), (4, 0), "{named}");
                assert!(task.example.is_some(), "{named}: the near end's given");
                let [near, far] = [sausage.spine[0], sausage.spine[SEGMENTS]];
                assert!(far[2] > near[2], "{named}: going back");
            }
            Bend::Strong => {
                assert_eq!((asked, caps), (5, 0), "{named}");
                let round = across.iter().map(|c| c.guide.exact.roundness());
                let opening = round.clone().fold(0.0, f64::max) - round.fold(1.0, f64::min);
                assert!(opening >= 0.3, "{named}: opening as it turns");
            }
            Bend::Crossing => {
                assert!((5..=6).contains(&asked) && caps == 2, "{named}");
                assert!(task.example.is_none());
                assert!(
                    task.outline.len() >= 2,
                    "{named}: passing in front of itself"
                );
            }
        }
    }
}

#[test]
fn each_section_lies_on_the_tube_and_its_ellipse_through_its_circle_s_image_within_1e_7() {
    let (mut sections, mut worst) = (0, 0.0_f64);
    for task in tasks() {
        let sausage = &task.sausage;
        for contour in every(&task) {
            for k in 0..64 {
                let p = contour.circle.at(k as f64 / 64.0 * std::f64::consts::TAU);
                // On the tube: on one capsule's surface, inside none.
                let off = (0..SEGMENTS)
                    .map(|j| {
                        let capsule = sausage.capsule(j);
                        norm(sub(p, capsule.nearest(p))) - capsule.radius
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(off.abs() < ON, "{:?}: off the tube by {off}", task.bend);
                let seen = task.view.at(p);
                let gap = off_ellipse(&contour.guide.exact, seen);
                worst = worst.max(gap);
                assert!(gap < EXACT, "{:?}: off its ellipse by {gap}", task.bend);
            }
            for p in contour.stretches.iter().flat_map(|s| &s.points) {
                assert!(off_ellipse(&contour.guide.exact, *p) < EXACT);
            }
            sections += 1;
        }
    }
    println!("{sections} sections, at most {worst:.1e} off");
}

#[test]
fn each_stretch_is_seen_or_hidden_as_the_depth_test_says() {
    let mut looked = 0;
    for task in tasks().step_by(2) {
        let sausage = &task.sausage;
        for contour in every(&task) {
            let tau = std::f64::consts::TAU;
            let circle: Vec<(f64, Point)> = (0..2048)
                .map(|k| {
                    let t = k as f64 / 2048.0 * tau;
                    (t, task.view.at(contour.circle.at(t)))
                })
                .collect();
            for stretch in &contour.stretches {
                let middle = stretch.points[stretch.points.len() / 2];
                let (t, _) = (circle.iter())
                    .min_by(|a, b| distance(a.1, middle).total_cmp(&distance(b.1, middle)))
                    .copied()
                    .expect("points");
                let p = contour.circle.at(t);
                let faces = dot(sub(p, contour.core), p) < 0.0;
                let unhidden = sausage.hit(p).is_none_or(|hit| hit.t >= 1.0 - 1e-6);
                assert_eq!(stretch.hidden, !(faces && unhidden), "{:?}", task.bend);
                looked += 1;
            }
        }
    }
    assert!(looked > 100);
}

#[test]
fn each_tick_lies_on_its_ellipse_where_the_traced_outline_runs() {
    for task in tasks() {
        let outline: Vec<&[Point]> = task.outline.iter().map(Vec::as_slice).collect();
        for contour in &task.contours {
            for [a, b] in &contour.ticks {
                let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
                assert!(off_ellipse(&contour.guide.exact, middle) < EXACT);
                // The outline is traced on a grid 0.005 apart: within a step and a half.
                let (_, off) = nearest(&outline, middle).expect("an outline");
                assert!(off < 0.008, "{:?}: a tick {off} off the outline", task.bend);
            }
        }
    }
}

#[test]
fn the_picture_is_the_tube_lit_on_paper_and_the_same_per_seed() {
    let task = ContourTask::generate(Bend::Strong, 5);
    assert_eq!(task, ContourTask::generate(Bend::Strong, 5));
    let picture = task.picture(200);
    assert_eq!(picture.len(), 200 * 200 * 4);
    let covered = picture.chunks(4).filter(|px| px[3] == 255).count();
    assert!(covered > 2_000, "the tube fills its place: {covered}");
    let middle = task.spine[task.spine.len() / 2];
    assert!(task.surface_at(middle).is_some());
    assert!(task.surface_at([0.01, 0.01]).is_none());
}

/// How far `p` lies from `ellipse`, to first order: its conic's value over its gradient's length
/// (exact enough for points within a hair of it, however flat it is).
fn off_ellipse(ellipse: &Ellipse, p: Point) -> f64 {
    let (sin, cos) = ellipse.angle.sin_cos();
    let [dx, dy] = [p[0] - ellipse.centre[0], p[1] - ellipse.centre[1]];
    let (x, y) = (dx * cos + dy * sin, -dx * sin + dy * cos);
    let [a, b] = ellipse.axes;
    let value = (x / a).powi(2) + (y / b).powi(2) - 1.0;
    let slope = (2.0 * x / (a * a)).hypot(2.0 * y / (b * b));
    value.abs() / slope.max(f64::MIN_POSITIVE)
}
