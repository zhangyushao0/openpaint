//! Texture's strip and its shadows (S154j): every strip keeps its rules seed after seed; a pebble's
//! shadow lies where its ball blocks the sun, by a ray test of the balls alone; a groove's shadow
//! lies inside it under the rim toward the sun, by its cross-section; the answer painted is matched whole with no mark
//! over light; an outline drawn round every form lies over light; at 稳 the side toward the light
//! keeps only its biggest shadows; a shadow the frame cuts is neither asked for nor missed (S158k).

use std::f64::consts::PI;

use super::field::{Bumps, Texture};
use super::rays::{GROOVE_SURFACES, Groove};
use super::shadows::TextureTask;
use crate::kinds::objects::object_body::GROUND;
use crate::kinds::tools::{DARKEST, DARKEST_PEN};
use crate::maths::vector::{add, dot, norm, sub, times};

/// Seeds looked at a variant; the copies' frame, pixels.
const SEEDS: u64 = 3;
const SIZE: [u32; 2] = [400, 200];

fn strips() -> Vec<TextureTask> {
    (Texture::ALL.iter())
        .flat_map(|texture| (0..SEEDS).map(|seed| TextureTask::generate(*texture, seed)))
        .collect()
}

/// A copy of `SIZE` painted the darkest where `dark` says, paper elsewhere.
fn painted(dark: impl Fn(usize) -> bool) -> Vec<u8> {
    let count = (SIZE[0] * SIZE[1]) as usize;
    (0..count)
        .flat_map(|at| {
            let [r, g, b] = if dark(at) { DARKEST } else { [255; 3] };
            [r, g, b, 255]
        })
        .collect()
}

#[test]
fn every_strip_keeps_its_rules() {
    for task in strips() {
        assert!(task.keeps(), "{:?}", task.texture);
        let shadows = task.shadows([200, 100]);
        assert!(
            (12..=30).contains(&shadows.blobs.len()),
            "{:?}",
            task.texture
        );
        // The eye tells shading from shadow: every lit side a step lighter than the shadow.
        let shades: Vec<(f64, bool)> = (0..200 * 100)
            .filter_map(|at| task.shade([(at % 200) as f64 / 200.0, (at / 200) as f64 / 200.0]))
            .map(|(_, shade, lit)| (shade, lit))
            .collect();
        let lit = shades.iter().filter(|(_, lit)| *lit).map(|(s, _)| *s);
        let dark = shades.iter().filter(|(_, lit)| !*lit).map(|(s, _)| *s);
        assert!(lit.fold(1.0, f64::min) - dark.fold(0.0, f64::max) >= 0.1);
    }
}

#[test]
fn a_pebble_s_shadow_lies_where_its_ball_blocks_the_sun() {
    for seed in 0..SEEDS {
        for texture in [Texture::ALL[0], Texture::ALL[1]] {
            let task = TextureTask::generate(texture, seed);
            let strip = &task.strip;
            let (balls, sun) = (&strip.scene.body.balls, strip.sun);
            // How far the sun's ray from `p` passes outside each ball (negative: through it).
            let passes = |p: [f64; 3], skip: Option<usize>| {
                (balls.iter().enumerate())
                    .filter(|(k, _)| Some(*k) != skip)
                    .filter_map(|(_, ball)| {
                        let w = sub(ball.centre, p);
                        let along = dot(w, sun);
                        (along > 0.0).then(|| norm(sub(w, times(sun, along))) - ball.radius)
                    })
                    .fold(f64::INFINITY, f64::min)
            };
            let mut looked = 0;
            for at in 0..200 * 100 {
                let p = [
                    (at % 200) as f64 / 200.0 + 0.0025,
                    (at / 200) as f64 / 200.0 + 0.0025,
                ];
                let seen = strip.seen(p).expect("the ground");
                let gap = passes(seen.at, None);
                if seen.surface != GROUND || gap.abs() < 1e-6 {
                    continue;
                }
                looked += 1;
                assert_eq!(strip.lit(&seen), gap > 0.0, "{texture:?} {seed} at {p:?}");
            }
            assert!(looked > 10_000);
        }
    }
}

#[test]
fn a_groove_s_shadow_lies_inside_it_under_the_rim_toward_the_sun() {
    let (mut grooves, mut shadowed) = (0, 0);
    for seed in 0..SEEDS {
        // The grooves alone, so no form's shadow falls in them.
        let mut task = TextureTask::generate(Texture::ALL[2], seed);
        let body = &mut task.strip.scene.body;
        body.balls.clear();
        body.rounds.clear();
        body.hulls.clear();
        let strip = &task.strip;
        for (k, groove) in strip.grooves.iter().enumerate() {
            let surface = GROOVE_SURFACES + 2 * k as u32;
            let mut looked = 0;
            for at in 0..400 * 200 {
                let p = [(at % 400) as f64 / 400.0, (at / 400) as f64 / 400.0];
                let Some(seen) = strip.seen(p).filter(|seen| seen.surface == surface) else {
                    continue;
                };
                let Some(lit) = in_light(groove, strip.sun, seen.at) else {
                    continue;
                };
                looked += 1;
                shadowed += usize::from(!lit);
                assert_eq!(strip.lit(&seen), lit, "{seed} groove {k} at {p:?}");
            }
            grooves += usize::from(looked > 0);
        }
    }
    assert!(
        grooves >= 3 && shadowed > 100,
        "{grooves} grooves, {shadowed} in shadow"
    );
}

/// Whether the trough's point `p` is lit, by the groove's cross-section alone: at the sun's
/// height `e` across it, the arc of `2e` from the rim away from the sun is lit and the rest, from
/// the rim toward the sun, in shadow. `None` at a rim, near that border, or where the sun's ray
/// from `p` would reach an end of the groove before its opening.
fn in_light(groove: &Groove, sun: [f64; 3], p: [f64; 3]) -> Option<bool> {
    let [along, side, up] = groove.local(p);
    let opening = along - up / sun[1] * dot(sun, groove.along);
    if up > -1e-6 || !(0.0..=groove.length).contains(&opening) {
        return None;
    }
    let across = groove.local(add(groove.start, sun))[1];
    let border = 2.0f64.mul_add(sun[1].atan2(across.abs()), -PI);
    let angle = up.atan2(side * across.signum());
    ((angle - border).abs() > 0.02).then_some(angle < border)
}

#[test]
fn the_answer_painted_matches_every_shadow_and_an_outline_lies_over_light() {
    for task in strips() {
        let shadows = task.shadows(SIZE);
        let answer = shadows.answer();
        let (check, picture) = task.read((SIZE, &painted(|at| answer[at])));
        assert_eq!(
            (check.matched, check.marks_over),
            (check.shadows, 0),
            "{:?}",
            task.texture
        );
        assert!(check.missed.is_empty() && check.over.is_empty());
        assert_eq!(picture.len(), answer.len() * 4);
        if task.texture.bumps != Bumps::Pebbles {
            continue;
        }
        // An outline a pen wide round every pebble, as the eye sees its ball.
        let unit = f64::from(SIZE[0]);
        let balls = &task.strip.scene.body.balls;
        let rings = |at: usize| {
            let p = [(at % 400) as f64 + 0.5, (at / 400) as f64 + 0.5].map(|v| v / unit);
            balls.iter().any(|ball| {
                let middle = task.strip.frame_point(ball.centre);
                let off = (p[0] - middle[0]).hypot(p[1] - middle[1]) - ball.radius * 1.15;
                off.abs() < DARKEST_PEN / 2.0
            })
        };
        let (outlined, _) = task.read((SIZE, &painted(rings)));
        assert!(
            outlined.marks_over >= 1 && !outlined.over.is_empty(),
            "{:?}: {} over light",
            task.texture,
            outlined.marks_over
        );
        assert!(outlined.matched < outlined.shadows, "{:?}", task.texture);
    }
}

#[test]
fn at_the_third_level_the_side_toward_the_light_keeps_its_biggest_shadows() {
    for texture in [Texture::ALL[4], Texture::ALL[5]] {
        for seed in 0..SEEDS {
            let task = TextureTask::generate(texture, seed);
            let shadows = task.shadows(SIZE);
            let toward = |k: usize| (shadows.middle(k)[0] >= 0.5) == texture.from_right;
            let whole = |k: &usize| !near_edge(&shadows.blobs[*k], shadows.size, false);
            let lit_side: Vec<usize> = (0..shadows.blobs.len())
                .filter(|k| toward(*k) && whole(k))
                .collect();
            let kept: Vec<usize> = lit_side
                .iter()
                .copied()
                .filter(|k| shadows.asked[*k])
                .collect();
            assert_eq!(kept.len(), lit_side.len().div_ceil(3), "{texture:?} {seed}");
            let least_kept = kept
                .iter()
                .map(|k| shadows.blobs[*k].len())
                .min()
                .unwrap_or(0);
            let dropped = lit_side.iter().filter(|k| !shadows.asked[**k]);
            assert!(
                dropped
                    .into_iter()
                    .all(|k| shadows.blobs[*k].len() <= least_kept)
            );
            // The other side keeps every one the frame leaves whole.
            assert!(
                (0..shadows.blobs.len())
                    .filter(|k| !toward(*k) && whole(k))
                    .all(|k| shadows.asked[k])
            );
        }
    }
}

/// Whether `blob` (pixels of a grid `[w, h]`, `SIZE`'s) comes within half a pen of the grid's
/// edge, or touches it (`touching`).
fn near_edge(blob: &[usize], [w, h]: [usize; 2], touching: bool) -> bool {
    let reach = if touching {
        1
    } else {
        (DARKEST_PEN / 2.0 * f64::from(SIZE[0])).ceil() as usize
    };
    blob.iter().any(|at| {
        let (x, y) = (at % w, at / w);
        x < reach || y < reach || x + reach >= w || y + reach >= h
    })
}

#[test]
fn a_shadow_the_frame_cuts_is_not_asked_for_nor_ringed_missed() {
    let mut cut = 0;
    for task in strips() {
        let shadows = task.shadows(SIZE);
        let edge: Vec<usize> = (0..shadows.blobs.len())
            .filter(|k| near_edge(&shadows.blobs[*k], shadows.size, true))
            .collect();
        cut += edge.len();
        assert!(
            edge.iter().all(|k| !shadows.asked[*k]),
            "{:?}",
            task.texture
        );
        // Its pixels lie outside the answer, and a copy leaving it bare misses nothing.
        let answer = shadows.answer();
        assert!((edge.iter()).all(|k| shadows.blobs[*k].iter().all(|at| !answer[*at])));
        let (check, _) = task.read((SIZE, &painted(|at| answer[at])));
        let middles: Vec<_> = edge.iter().map(|k| shadows.middle(*k)).collect();
        assert!(check.missed.iter().all(|at| !middles.contains(at)));
        assert_eq!(check.matched, check.shadows, "{:?}", task.texture);
    }
    assert!(cut > 0, "some strip's shadow runs past the frame");
}
