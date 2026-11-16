//! Form intersections' exactness (S156k): the scene rules hold seed after seed; every sample of the
//! meeting line, every pierce point and every point the sections find lies on both forms' surfaces
//! within 1e-7; its stretches are seen and hidden as the depth test says.

use super::forms::Form;
use super::meeting::{IntersectionTask, Meeting, room};
use crate::maths::path::{Point, distance, length};
use crate::maths::random::derived;
use crate::maths::vector::Vector;

/// Tasks of each variant the tests lay.
const EACH: u64 = 12;
/// Within this (world units) a point lies on a surface.
const EXACT: f64 = 1e-7;

fn tasks() -> impl Iterator<Item = IntersectionTask> {
    Meeting::ALL.into_iter().flat_map(|meeting| {
        (0..EACH).map(move |k| IntersectionTask::generate(meeting, derived(0x0156_1A75, k)))
    })
}

/// How far `p` (the world) lies off each form's surface.
fn off(task: &IntersectionTask, p: Vector) -> [f64; 2] {
    let local = task.placed.local(p);
    task.forms.map(|form| form.off(local).abs())
}

#[test]
fn the_scene_rules_hold_seed_after_seed() {
    for task in tasks() {
        let meeting = task.meeting;
        assert!(task.seen.keeps(false), "{meeting:?}: the box's rules");
        let front = task.seen.front_angle();
        assert!((100.0..=150.0).contains(&front), "{meeting:?}: {front}");
        let inside = |p: Point| (0..2).all(|k| (0.04..=0.96).contains(&p[k]));
        for form in &task.forms {
            let points = form.hull_points(task.placed).into_iter();
            assert!(points.map(|p| task.view.at(p)).all(inside), "{meeting:?}");
        }
        let seen: f64 = (task.answer.iter())
            .filter(|s| !s.hidden)
            .map(|s| length(&s.points))
            .sum();
        assert!(seen >= 0.15, "{meeting:?}: {seen}");
        let least = if meeting == Meeting::Corner {
            0.03
        } else {
            0.05
        };
        for (p, pierced) in &task.met.pierce {
            if task.sees(*p) {
                let outline = task.outline(*pierced);
                assert!(room(&outline, task.view.at(*p)) >= least, "{meeting:?}");
            }
        }
        // The forms each level asks for.
        let kinds = task.forms.map(|form| match form {
            Form::Box(_) => 'b',
            Form::Cylinder(..) => 'c',
            Form::Sphere(..) => 's',
        });
        let expected = match meeting {
            Meeting::Edge | Meeting::Corner => ['b', 'b'],
            Meeting::CylinderIntoBox => ['b', 'c'],
            Meeting::BoxIntoCylinder => ['c', 'b'],
            Meeting::Cylinders => ['c', 'c'],
            Meeting::Sphere => ['c', 's'],
        };
        assert_eq!(kinds, expected);
        assert!(!task.pierced.is_empty(), "{meeting:?}");
    }
}

#[test]
fn every_point_of_the_meeting_line_lies_on_both_surfaces_within_1e_7() {
    let (mut samples, mut worst) = (0, 0.0_f64);
    for task in tasks() {
        let meeting = task.meeting;
        for track in &task.met.tracks {
            let (span, _) = track.span();
            for k in 0..=200 {
                let p = track.at(span * k as f64 / 200.0);
                let [a, b] = off(&task, p);
                worst = worst.max(a.max(b));
                assert!(a < EXACT && b < EXACT, "{meeting:?}: off by {a:e}, {b:e}");
                samples += 1;
            }
        }
        let found = task
            .met
            .pierce
            .iter()
            .map(|(p, _)| p)
            .chain(&task.met.found);
        for p in found {
            let [a, b] = off(&task, *p);
            assert!(
                a < EXACT && b < EXACT,
                "{meeting:?}: a point found off by {a:e}, {b:e}"
            );
        }
    }
    println!("{samples} samples, at most {worst:.1e} off");
}

#[test]
fn each_stretch_of_the_meeting_line_is_seen_or_hidden_as_the_depth_test_says() {
    let mut looked = 0;
    for task in tasks().step_by(2) {
        let points: Vec<(Point, Vector)> = (task.met.tracks.iter())
            .flat_map(|track| {
                let (span, _) = track.span();
                (0..=2000).map(move |k| track.at(span * k as f64 / 2000.0))
            })
            .map(|p| (task.view.at(p), p))
            .collect();
        for stretch in &task.answer {
            let middle = stretch.points[stretch.points.len() / 2];
            let near: Vec<&(Point, Vector)> = (points.iter())
                .filter(|(seen, _)| distance(*seen, middle) < 2e-3)
                .collect();
            // Where two pieces of the line cross in the picture, either may be the one.
            let Some((_, p)) = near.first() else { continue };
            if near.iter().any(|(_, q)| distance_3(*p, *q) > 0.05) {
                continue;
            }
            assert_eq!(stretch.hidden, !task.sees(*p), "{:?}", task.meeting);
            looked += 1;
        }
    }
    assert!(looked > 50, "{looked}");
}

fn distance_3(a: Vector, b: Vector) -> f64 {
    crate::maths::vector::norm(crate::maths::vector::sub(a, b))
}

#[test]
fn a_task_is_the_same_per_seed() {
    for meeting in Meeting::ALL {
        let task = IntersectionTask::generate(meeting, 9);
        assert_eq!(task, IntersectionTask::generate(meeting, 9));
    }
}
