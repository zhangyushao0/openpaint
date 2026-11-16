//! Light on form's model on its constructions (S153): the terminator where the construction puts
//! it within 1e-7 for each solid; the cast shadow the silhouette's shadow; the light read back from
//! the pin within 1e-7; scenes keeping their rules seed after seed; the answer painted reads
//! accurate; each reason fires alone on a copy made wrong one way.

use super::answer::{Grey, Mass, Shading};
use super::scene::{Form, LightTask, Lit, Tones};
use crate::geometry::light::{Light, box_shadow, sun_read};
use crate::marks::compared::{Outcome, Verdict};
use crate::maths::path::{Point, distance, nearest_on_segment};
use crate::maths::vector::{Vector, across, add, dot, norm, sub, times, unit};

/// Seeds looked at a variant.
const SEEDS: u64 = 4;
/// The frame's side the copies are painted at, pixels; how far either side of the terminator a
/// copy made wrong there is painted the other family (frame units: past the soft band, wider than
/// a grey pen).
const SIDE: u32 = 400;
const BAND: f64 = 0.12;
/// How far a copy made wrong moves the cast shadow (frame units).
const STEP: f64 = 0.1;

/// The tasks of `forms`, `SEEDS` each, a variant of each tone.
fn tasks(forms: &[Form]) -> Vec<LightTask> {
    let lits = Lit::ALL.iter().filter(|lit| forms.contains(&lit.form));
    lits.flat_map(|lit| (0..SEEDS).map(|seed| LightTask::generate(*lit, seed)))
        .collect()
}

/// The way to the sun.
fn sun(task: &LightTask) -> Vector {
    let Light::Sun(way) = task.light else {
        panic!("a sun");
    };
    way
}

/// The form's normal at its surface point `p` facing outward along `out` (about), as the model
/// finds it: the ray arriving there from just outside.
fn normal_at(task: &LightTask, p: Vector, out: Vector) -> Vector {
    let from = add(p, times(out, 1e-9));
    let hit = task
        .scene
        .body
        .cast((from, times(out, -1.0)))
        .expect("on the form");
    hit.normal
}

#[test]
fn the_terminator_lies_where_its_construction_puts_it() {
    for task in tasks(&[Form::Cylinder, Form::Cone]) {
        let round = task.scene.body.rounds[0];
        let sides = task.side_terminator().expect("a terminator");
        let centre = round.base.centre();
        for [foot, end] in sides {
            let p = add(foot, times(sub(end, foot), 0.25));
            let from_axis = sub(
                p,
                add(centre, times(round.axis, dot(sub(p, centre), round.axis))),
            );
            let turned = dot(normal_at(&task, p, unit(from_axis)), sun(&task));
            assert!(turned.abs() < 1e-7, "{:?}: {turned}", task.lit);
        }
        if round.cone {
            // The lines from the point's shadow touch the base where the terminator meets it.
            let point = add(centre, times(round.axis, round.length));
            let shadow = task
                .light
                .shadow(point, task.scene.ground)
                .expect("a shadow");
            for [touch, _] in sides {
                let [radius, line] = [sub(touch, centre), sub(touch, shadow)];
                let off = dot(unit(radius), unit(line));
                assert!(off.abs() < 1e-7, "{:?}: {off}", task.lit);
            }
        }
    }
    for task in tasks(&[Form::Sphere]) {
        let ball = task.scene.body.balls[0];
        let [u, v] = across(sun(&task));
        for k in 0..12 {
            let t = f64::from(k) / 12.0 * std::f64::consts::TAU;
            let out = add(times(u, t.cos()), times(v, t.sin()));
            let p = add(ball.centre, times(out, ball.radius));
            let turned = dot(normal_at(&task, p, out), sun(&task));
            assert!(turned.abs() < 1e-7, "{:?}: {turned}", task.lit);
        }
    }
    for task in tasks(&[Form::Box]) {
        // Each shadow corner where the ray through its top corner meets the ground line from the
        // corner under it, along the shadows' way.
        let outline = box_shadow(&task.light, &task.corners, task.scene.ground).expect("a shadow");
        let way = sun(&task);
        for (shadow, index) in outline.into_iter().filter(|(_, index)| index & 2 != 0) {
            let (top, foot) = (task.corners[index], task.corners[index & !2]);
            let ray = norm(crate::maths::vector::cross(sub(top, shadow), way));
            let ground = sub(shadow, foot);
            let along = ground[0] * way[2] - ground[2] * way[0];
            assert!(
                ray < 1e-7 && along.abs() < 1e-7,
                "{:?}: {ray} {along}",
                task.lit
            );
        }
    }
}

/// `points` (on the ground) round their outline in x and z.
fn hull(points: &[Vector]) -> Vec<Vector> {
    let flat: Vec<Point> = points.iter().map(|p| [p[0], p[2]]).collect();
    let round = crate::kinds::cast_shadows::answer::convex(flat);
    let height = points.first().map_or(0.0, |p| p[1]);
    round.into_iter().map(|[x, z]| [x, height, z]).collect()
}

#[test]
fn the_cast_shadow_is_the_silhouette_s_shadow() {
    for task in tasks(&[Form::Box, Form::Cylinder, Form::Cone, Form::Sphere]) {
        let (ground, light) = (task.scene.ground, task.light);
        let body = &task.scene.body;
        let turn = |k: usize| k as f64 / 96.0 * std::f64::consts::TAU;
        let mut silhouette: Vec<Vector> = Vec::new();
        if let Some(ball) = body.balls.first() {
            let [u, v] = across(sun(&task));
            silhouette.extend((0..96).map(|k| {
                let out = add(times(u, turn(k).cos()), times(v, turn(k).sin()));
                add(ball.centre, times(out, ball.radius))
            }));
        } else if let Some(round) = body.rounds.first() {
            for square in [round.base, round.top()] {
                silhouette.extend((0..96).map(|k| square.rim(turn(k))));
            }
            // The side's terminator exactly: where its shadow's sides touch the ends'.
            silhouette.extend(task.side_terminator().into_iter().flatten().flatten());
        } else {
            silhouette.extend(task.corners);
        }
        // What stands on or near the ground throws its shadow on the form's own footprint.
        let under: Vec<Vector> = (silhouette.iter().copied())
            .filter(|p| p[1] - ground < 0.05)
            .filter_map(|p| light.shadow(p, ground))
            .collect();
        let shadows: Vec<Vector> = (silhouette.iter())
            .filter_map(|p| light.shadow(*p, ground))
            .collect();
        let outline = hull(&shadows);
        let count = outline.len();
        for k in 0..count {
            let at = outline[k];
            let [before, after] = [outline[(k + count - 1) % count], outline[(k + 1) % count]];
            if under.iter().any(|p| norm(sub(*p, at)) < 1e-9) {
                continue;
            }
            let along = sub(after, before);
            let out = unit([along[2], 0.0, -along[0]]);
            let up = [0.0, 1.0, 0.0];
            let lit = |p: Vector| task.scene.lit((p, up), light.from(p));
            let (inside, outside) = (add(at, times(out, -1e-5)), add(at, times(out, 1e-5)));
            // Where the form lies over the ground (a lying cylinder's side), no shadow ends.
            if body.cast((outside, up)).is_some() {
                continue;
            }
            assert!(!lit(inside) && lit(outside), "{:?}: {at:?}", task.lit);
        }
    }
}

#[test]
fn the_light_reads_back_from_the_pin() {
    for lit in Lit::ALL {
        for seed in 0..SEEDS {
            let task = LightTask::generate(lit, seed);
            let read = sun_read(
                (task.view.scale, task.view.offset),
                task.scene.ground,
                task.pinned,
            );
            let off = norm(sub(read.expect("read"), sun(&task)));
            assert!(off < 1e-7, "{lit:?} {seed}: {off}");
        }
    }
}

#[test]
fn every_scene_keeps_its_rules_seed_after_seed() {
    for lit in Lit::ALL {
        for seed in 0..SEEDS {
            let task = LightTask::generate(lit, seed);
            assert!(task.keeps(), "{lit:?} {seed}");
        }
    }
}
/// The copy whose every pixel `paint` gives the grey of, from the model's shading there.
fn painted(task: &LightTask, paint: impl Fn(Point, &Shading) -> Grey) -> Vec<u8> {
    let map = task.map([SIDE; 2]);
    let side = SIDE as usize;
    (0..side * side)
        .flat_map(|at| {
            let p = [at % side, at / side].map(|v| (v as f64 + 0.5) / f64::from(SIDE));
            let grey = map[at].as_ref().map_or(Grey::Light, |s| paint(p, s));
            let [r, g, b] = grey.grey();
            [r, g, b, 255]
        })
        .collect()
}

#[test]
fn the_answer_painted_reads_accurate() {
    for lit in Lit::ALL {
        let task = LightTask::generate(lit, 1);
        let picture = task.picture(SIDE);
        let check = task.read(([SIDE; 2], &picture));
        assert!(check.families && check.reason.is_none(), "{lit:?}");
        assert_eq!(check.verdict(), Verdict::App(Outcome::Clean), "{lit:?}");
        assert!(check.share > 0.98, "{lit:?}: {}", check.share);
    }
}

/// The reason `task`'s copy painted by `paint` reads, and whether the families hold.
fn reason(task: &LightTask, paint: impl Fn(Point, &Shading) -> Grey) -> (bool, String) {
    let copy = painted(task, paint);
    let check = task.read(([SIDE; 2], &copy));
    let words = check
        .reason
        .map(|say| format!("{:?}", say.args))
        .unwrap_or_default();
    (check.families, words)
}

/// A way of painting a copy: each pixel's grey from where it lies and the model's shading there.
type Paint<'a> = Box<dyn Fn(Point, &Shading) -> Grey + 'a>;

/// `task`'s copies made wrong one way each, with the reason each should read: the terminator a
/// band toward the light, the whole shadow side light, the cast shadow moved along the pin's
/// shadow, back toward the form, and across.
fn wrong_ways(task: &LightTask) -> [(&'static str, Paint<'_>); 5] {
    let terminator = task.parting().terminator;
    let near_terminator = move |p: Point| {
        (terminator.iter()).any(|line| {
            (line.windows(2)).any(|w| distance(nearest_on_segment(w[0], w[1], p), p) < BAND)
        })
    };
    let toward_light = move |p: Point, s: &Shading| {
        let darkened = s.on_form && s.family == Mass::Light && near_terminator(p);
        if darkened { Grey::Dark } else { s.tone }
    };
    let toward_shadow = |_: Point, s: &Shading| {
        let lightened = s.on_form && s.family == Mass::Shadow;
        if lightened { Grey::Light } else { s.tone }
    };
    let [foot, _, tip] = task.pinned;
    let long = distance(foot, tip);
    let way = [
        (tip[0] - foot[0]) / long * STEP,
        (tip[1] - foot[1]) / long * STEP,
    ];
    let moved = |by: Point| -> Paint<'_> {
        Box::new(move |p: Point, s: &Shading| {
            if s.on_form {
                return s.tone;
            }
            let cast = (task.shading([p[0] - by[0], p[1] - by[1]])).is_some_and(|s| s.cast());
            if cast { Grey::Dark } else { Grey::Light }
        })
    };
    [
        ("terminator-light", Box::new(toward_light)),
        ("terminator-shadow", Box::new(toward_shadow)),
        ("cast-long", moved(way)),
        ("cast-short", moved([-way[0], -way[1]])),
        ("cast-turned", moved([-way[1], way[0]])),
    ]
}

#[test]
fn each_reason_fires_alone() {
    // A box lit from the side and a sphere lit from behind, their halftones too: the first scene
    // whose form shows a shadow side wide enough to paint wrong (a fortieth of the frame).
    for lit in [Lit::ALL[3], Lit::ALL[10]] {
        let wide = |task: &LightTask| {
            let map = task.map([SIDE / 4; 2]);
            let shadow = (map.iter().flatten()).filter(|s| s.on_form && s.family == Mass::Shadow);
            shadow.count() * 40 >= map.len()
        };
        let task = (0..40)
            .map(|seed| LightTask::generate(lit, seed))
            .find(wide)
            .expect("a wide shadow side");
        assert_ne!(task.lit.tones, Tones::Four);
        for (expected, paint) in wrong_ways(&task) {
            let (families, words) = reason(&task, paint);
            assert!(
                !families && words.contains(expected),
                "{lit:?} {expected}: {words}"
            );
        }
    }
}
