//! Cast shadows' constructions on their answers (S153): every shadow corner where its ray and its
//! ground line meet within 1e-7, the same point the light's projection throws; on a side it climbs,
//! straight above where its ground line meets the side; scenes keep their rules seed after seed;
//! Compare lays the same whatever the copy holds.

use super::scene::{ShadowTask, Throw};
use crate::compositions::levels::Level;
use crate::geometry::light::Light;
use crate::kinds::Original;
use crate::marks::asked::{Asked, Compare, Copied};
use crate::marks::card::Choices;
use crate::marks::compared::{DrawnCopy, Seen};
use crate::maths::colour::Rgb8;
use crate::maths::path::{Point, distance, meet};
use crate::sheets::exercise::Exercise;

/// Seeds looked at a variant.
const SEEDS: u64 = 12;

/// Where the lines through `a` along `along` and through `b` along `by` meet.
fn crossing((a, along): (Point, Point), (b, by): (Point, Point)) -> Point {
    let line = |p: Point, d: Point| [p, [p[0] + d[0], p[1] + d[1]]];
    meet(line(a, along), line(b, by)).expect("not parallel")
}

#[test]
fn every_shadow_corner_lies_where_its_ray_meets_its_ground_line() {
    for throw in Throw::ALL {
        for seed in 0..SEEDS {
            let task = ShadowTask::generate(throw, seed);
            assert!(!task.shadows.corners.is_empty(), "{throw:?} {seed}");
            for corner in &task.shadows.corners {
                let found = match (task.points, task.light) {
                    // Parallel to the picture: the rays at the light's slant, the ground lines level.
                    (None, Light::Sun([x, y, _])) => {
                        crossing((corner.top, [x, -y]), (corner.foot, [1.0, 0.0]))
                    }
                    (Some([light, ground]), _) => {
                        let ray = [light, corner.top];
                        let foot = corner.upright.map_or(corner.foot, |upright| {
                            // The ground line runs through the foot to the side.
                            let line = [ground, corner.foot];
                            let off = distance(
                                meet(line, [upright, [upright[0], 0.0]]).expect("met"),
                                upright,
                            );
                            assert!(off < 1e-7, "{throw:?} {seed}: {off}");
                            upright
                        });
                        let up = corner.upright.is_some();
                        let other = if up {
                            [foot, [foot[0], foot[1] - 1.0]]
                        } else {
                            [ground, foot]
                        };
                        meet(ray, other).expect("not parallel")
                    }
                    _ => panic!("a lamp's points are given"),
                };
                let off = distance(found, corner.shadow);
                assert!(off < 1e-7, "{throw:?} {seed}: {off}");
            }
        }
    }
}

#[test]
fn every_scene_keeps_its_rules_seed_after_seed() {
    for throw in Throw::ALL {
        for seed in 0..SEEDS {
            let task = ShadowTask::generate(throw, seed);
            assert!(task.keeps(), "{throw:?} {seed}");
            let hidden = task
                .shadows
                .outline
                .iter()
                .filter(|(_, hidden)| *hidden)
                .count();
            assert!(
                task.shadows.outline.len() > hidden,
                "{throw:?} {seed}: something seen"
            );
        }
    }
    // A climbing shadow climbs: some of its outline stands above the ground's.
    let climbing = ShadowTask::generate(Throw::Climbing, 3);
    assert_eq!(climbing.shadows.climbing, Some(true));
    assert!(climbing.shadows.corners.iter().any(|c| c.upright.is_some()));
}

/// A copy of `lines`, nothing painted.
struct Lines(Vec<Vec<Point>>);

impl Copied for Lines {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        Vec::new()
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy {
            all: self.0.clone(),
        }
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        Vec::new()
    }
}

#[test]
fn compare_lays_the_same_answer_whatever_the_copy_holds() {
    let exercise = Exercise::generated(crate::kinds::Kind::Shadows(Throw::Front), 5);
    let task = ShadowTask::generate(Throw::Front, 5);
    let answer = task.answer();
    let laid = |copy: &Lines| {
        let asked = Asked {
            exercise: &exercise,
            original: &task,
            picture: None,
            copy,
            choices: Choices::default(),
        };
        let Some(Compare::Checked(compared)) = task.compare(&asked) else {
            panic!("checked");
        };
        let seen = Seen {
            construction: true,
            shown: 1.0,
            chosen: 0,
            outline: &[],
            grey: false,
            view: 0,
            fit: None,
            probe: None,
        };
        (
            compared.marks(&seen),
            compared.card(&seen),
            compared.verdict(),
        )
    };
    let exact = laid(&Lines(answer.clone()));
    let off = laid(&Lines(vec![vec![[0.1, 0.9], [0.8, 0.2]]]));
    assert_eq!(exact, off);
}
