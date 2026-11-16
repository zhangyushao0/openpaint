//! A sausage in space (S156k, Drawabox's organic forms): a tube of one radius swept along a smooth
//! spine (`geometry/space_path`), its ends hemispheres. It is built as a chain of capsules along
//! the spine sampled at even lengths (`SEGMENTS`), whose ray hit is exact (`capsule_hit`): the
//! model every line of the exercise lies on. A section lies in the middle of one capsule, square
//! to it, so its circle lies on the model exactly; an end's lies on its ball.
//!
//! The world: y up, z away from the eye, which looks level from the origin.

use crate::geometry::space_path::SpacePath;
use crate::kinds::objects::object_body::{Capsule, Hit};
use crate::kinds::objects::object_rays::capsule_hit;
use crate::maths::vector::{Vector, add, dot, norm, sub, times};

/// Capsules along the spine; capsules a bounding ball holds.
pub(crate) const SEGMENTS: usize = 96;
const GROUP: usize = 8;

/// A tube along a spine: the spine's points (`SEGMENTS` + 1), each with the frame's normal there
/// (rotation-minimising, from the world's up), and the tube's radius.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sausage {
    pub(crate) spine: Vec<Vector>,
    pub(crate) normals: Vec<Vector>,
    pub(crate) radius: f64,
    /// Each run of `GROUP` capsules' bounding ball: its centre and radius.
    groups: Vec<(Vector, f64)>,
}

impl Sausage {
    /// The tube of `radius` along `path`.
    pub(crate) fn along(path: &SpacePath, radius: f64) -> Self {
        let samples = path.samples(SEGMENTS + 1, [0.0, 1.0, 0.0], &[]);
        let spine: Vec<Vector> = samples.iter().map(|sample| sample.at).collect();
        let normals = samples.iter().map(|sample| sample.normal).collect();
        let groups = (0..SEGMENTS)
            .step_by(GROUP)
            .map(|first| {
                let held = &spine[first..=(first + GROUP).min(SEGMENTS)];
                let [a, b] = [held[0], held[held.len() - 1]];
                let centre = times(add(a, b), 0.5);
                let reach = held.iter().map(|p| norm(sub(*p, centre)));
                (centre, reach.fold(0.0, f64::max) + radius)
            })
            .collect();
        Self {
            spine,
            normals,
            radius,
            groups,
        }
    }

    /// Capsule `k`, from spine point `k` to the next.
    pub(crate) fn capsule(&self, k: usize) -> Capsule {
        Capsule {
            ends: [self.spine[k], self.spine[k + 1]],
            radius: self.radius,
        }
    }

    /// The spine's length.
    pub(crate) fn length(&self) -> f64 {
        (self.spine.windows(2))
            .map(|pair| norm(sub(pair[1], pair[0])))
            .sum()
    }

    /// What the ray from `origin` along `d` meets first, `t` 1 at `origin + d`, and its capsule.
    pub(crate) fn cast(&self, (origin, d): (Vector, Vector)) -> Option<(Hit, usize)> {
        let near = |(centre, reach): &(Vector, f64)| {
            let w = sub(*centre, origin);
            let t = (dot(w, d) / dot(d, d)).max(0.0);
            norm(sub(w, times(d, t))) <= *reach
        };
        let mut best: Option<(Hit, usize)> = None;
        for (group, _) in self.groups.iter().enumerate().filter(|(_, g)| near(g)) {
            let first = group * GROUP;
            for k in first..(first + GROUP).min(SEGMENTS) {
                let surface = u32::try_from(k).unwrap_or(u32::MAX);
                let Some(hit) = capsule_hit(&self.capsule(k), (origin, d), surface) else {
                    continue;
                };
                if best.is_none_or(|(held, _)| hit.t < held.t) {
                    best = Some((hit, k));
                }
            }
        }
        best
    }

    /// What the eye's ray towards `p` meets first, `t` 1 at `p`.
    pub(crate) fn hit(&self, p: Vector) -> Option<Hit> {
        self.cast(([0.0; 3], p)).map(|(hit, _)| hit)
    }

    /// Whether the eye sees `p`, a point of the surface: nothing nearer on its ray.
    pub(crate) fn sees(&self, p: Vector) -> bool {
        (self.hit(p)).is_none_or(|hit| hit.t >= 1.0 - super::seen::SEEN)
    }

    /// The least radius the spine turns by between its capsules (their turn over their length).
    pub(crate) fn tightest(&self) -> f64 {
        (self.spine.windows(3))
            .map(|three| {
                let [a, b] = [sub(three[1], three[0]), sub(three[2], three[1])];
                let turn = (dot(a, b) / (norm(a) * norm(b))).clamp(-1.0, 1.0).acos();
                (norm(a) + norm(b)) / 2.0 / turn.max(f64::MIN_POSITIVE)
            })
            .fold(f64::INFINITY, f64::min)
    }

    /// The least distance between two points of the spine this far apart along it at least: the
    /// tube never meets itself where it is `2 × radius` and more.
    pub(crate) fn closest_apart(&self, along: f64) -> f64 {
        let step = self.length() / SEGMENTS as f64;
        let gap = (along / step).ceil() as usize;
        let mut least = f64::INFINITY;
        for i in 0..self.spine.len() {
            for j in (i + gap)..self.spine.len() {
                least = least.min(norm(sub(self.spine[j], self.spine[i])));
            }
        }
        least
    }
}
