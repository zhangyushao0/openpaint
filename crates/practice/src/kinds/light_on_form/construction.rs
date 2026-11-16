//! Light on form's way to see the answer (看画法, S153): the light's direction through the form.
//! A box's faces turned away end at the edges where a lit face meets an unlit one; its cast
//! shadow by the rays through its top corners and the ground lines from the corners under them,
//! along the pin's shadow, meeting at the shadow's corners. A cylinder's end cut by the diameter
//! square to the light, the side's terminator dropped from its ends; a cone's terminator from its
//! point to where the lines from its point's shadow touch its base; a sphere's great circle square
//! to the light, the ray through its highest point and the ground line from where it stands. A
//! carved object's as its box's. Each line in frame units, dashed where the form hides it.

use crate::geometry::light::box_shadow;
use crate::kinds::light_on_form::scene::{Form, LightTask};
use crate::maths::path::Point;
use crate::maths::vector::{Vector, across, add, dot, norm, sub, times, unit};

/// A great circle drawn in this many pieces.
const CIRCLE: usize = 96;

/// A construction line: its points in frame units, and whether the form hides it (dashed).
pub type Drawn = (Vec<Point>, bool);

impl LightTask {
    /// The way to see the answer: lines in frame units, dashed where hidden.
    #[must_use]
    pub fn construction(&self) -> Vec<Drawn> {
        let mut lines: Vec<Drawn> = (self.creases.iter())
            .map(|crease| (crease.to_vec(), false))
            .collect();
        match self.lit.form {
            Form::Box | Form::Object => lines.extend(self.box_rays()),
            Form::Cylinder => lines.extend(self.cylinder()),
            Form::Cone => lines.extend(self.cone()),
            Form::Sphere => lines.extend(self.sphere()),
        }
        lines
    }

    /// The segment from `a` to `b` in the world, in the frame.
    fn segment(&self, a: Vector, b: Vector) -> Drawn {
        (vec![self.view.at(a), self.view.at(b)], false)
    }

    /// The form's box's rays through its top corners whose shadows are its shadow's corners, and
    /// the ground lines from the corners under them.
    fn box_rays(&self) -> Vec<Drawn> {
        let ground = self.scene.ground;
        let Some(outline) = box_shadow(&self.light, &self.corners, ground) else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        for (shadow, index) in outline.iter().filter(|(_, index)| index & 2 != 0) {
            let (top, foot) = (self.corners[*index], self.corners[index & !2]);
            lines.push(self.segment(top, *shadow));
            lines.push(self.segment(foot, *shadow));
        }
        lines
    }

    /// The points where a round part's side turns from the light on its two ends (its base's and
    /// its top's), by their turn round the axis; none where the light runs along the axis.
    pub(crate) fn side_terminator(&self) -> Option<[[Vector; 2]; 2]> {
        let round = self.scene.body.rounds.first()?;
        let (towards, _) = self.light.from(round.base.centre());
        let [u, v] = across(round.axis);
        let (a, b) = (dot(u, towards), dot(v, towards));
        if a.hypot(b) < 1e-12 {
            return None;
        }
        // The side's outward way `u cos t + v sin t` square to the light, for a cylinder; a
        // cone's surface leans by its slope (`terminator_of_cone`).
        let r = round.radius();
        let lean = if round.cone {
            r * dot(round.axis, towards) / round.length
        } else {
            0.0
        };
        let reach = a.hypot(b);
        let k = (-lean / reach).clamp(-1.0, 1.0);
        let (phi, spread) = (b.atan2(a), k.acos());
        let centre = round.base.centre();
        let top = add(centre, times(round.axis, round.length));
        Some([phi - spread, phi + spread].map(|t| {
            let out = add(times(u, r * t.cos()), times(v, r * t.sin()));
            let foot = add(centre, out);
            let end = if round.cone {
                top
            } else {
                add(foot, times(round.axis, round.length))
            };
            [foot, end]
        }))
    }

    /// A cylinder's diameter square to the light on its top end, the side's terminator dropped
    /// from its ends, the rays through those ends and the ground lines from the side's feet where
    /// they stand on the ground.
    fn cylinder(&self) -> Vec<Drawn> {
        let Some(sides) = self.side_terminator() else {
            return Vec::new();
        };
        let ground = self.scene.ground;
        let mut lines = vec![self.segment(sides[0][1], sides[1][1])];
        for [foot, end] in sides {
            lines.push(self.segment(foot, end));
            for (from, to) in [(end, foot), (foot, end)] {
                let Some(shadow) = self.light.shadow(from, ground) else {
                    continue;
                };
                if norm(sub(shadow, from)) > 1e-9 {
                    lines.push(self.segment(from, shadow));
                    if (to[1] - ground).abs() <= 1e-9 && from != foot {
                        lines.push(self.segment(to, shadow));
                    }
                }
            }
        }
        lines
    }

    /// A cone's point's ray to its shadow, the lines from that shadow touching the base, and the
    /// terminator from the point to where they touch.
    fn cone(&self) -> Vec<Drawn> {
        let (Some(round), Some(sides)) = (self.scene.body.rounds.first(), self.side_terminator())
        else {
            return Vec::new();
        };
        let point = add(round.base.centre(), times(round.axis, round.length));
        let Some(shadow) = self.light.shadow(point, self.scene.ground) else {
            return Vec::new();
        };
        let mut lines = vec![self.segment(point, shadow)];
        for [touch, _] in sides {
            lines.push(self.segment(shadow, touch));
            lines.push(self.segment(point, touch));
        }
        lines
    }

    /// A sphere's great circle square to the light, dashed where it turns from the eye; the ray
    /// through its highest point and the ground line from where the sphere stands.
    fn sphere(&self) -> Vec<Drawn> {
        let Some(ball) = self.scene.body.balls.first() else {
            return Vec::new();
        };
        let (towards, _) = self.light.from(ball.centre);
        let [u, v] = across(unit(towards));
        let at = |k: usize| {
            let t = k as f64 / CIRCLE as f64 * std::f64::consts::TAU;
            let out = add(times(u, t.cos()), times(v, t.sin()));
            add(ball.centre, times(out, ball.radius))
        };
        let mut lines: Vec<Drawn> = Vec::new();
        for k in 0..CIRCLE {
            let [a, b] = [at(k), at(k + 1)];
            // Seen where it faces the eye at the origin.
            let hidden = dot(sub(a, ball.centre), times(a, -1.0)) < 0.0;
            match lines.last_mut() {
                Some((points, was)) if *was == hidden => points.push(self.view.at(b)),
                _ => lines.push((vec![self.view.at(a), self.view.at(b)], hidden)),
            }
        }
        let highest = (0..CIRCLE)
            .map(at)
            .max_by(|a, b| a[1].total_cmp(&b[1]))
            .unwrap_or(ball.centre);
        let ground = self.scene.ground;
        if let Some(shadow) = self.light.shadow(highest, ground) {
            let stands = sub(ball.centre, [0.0, ball.radius, 0.0]);
            lines.push(self.segment(highest, shadow));
            lines.push(self.segment(stands, shadow));
        }
        lines
    }
}
