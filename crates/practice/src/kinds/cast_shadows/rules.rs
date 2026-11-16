//! Cast shadows' scene rules (S153): a scene is laid again from its seed's sequence until it keeps
//! every one. The boxes keep the boxes' rules and lie apart; every shadow inside the frame, each top
//! corner's shadow at least `LONG` from its corner, at least `SEEN` of the outline seen past the
//! boxes; no shadow on the other box or the other's shadow, but where it climbs; the pin and its
//! shadow, or the lamp and its foot, clear of the boxes and their shadows and inside the frame; a
//! sun's points on the paper beside the frame (`PAPER`), where the eye aims at them.

use super::answer::convex;
use super::scene::{ShadowTask, Throw};
use crate::geometry::perspective::{FAR, PerspectiveBox};
use crate::maths::path::{Point, apart, distance};

/// The horizon's height in the frame, at least and at most.
const HORIZON: [f64; 2] = [0.04, 0.4];
/// Every shadow point inside the frame by this; the lamp and pin's points by this.
const INSIDE: f64 = 0.03;
/// Each top corner's shadow at least this far from its corner on the picture.
const LONG: f64 = 0.05;
/// The boxes, the shadows, the pin and the lamp's foot at least this far apart on the picture.
const APART: f64 = 0.03;
/// A box's front angle, degrees (V大's, as the boxes kind's), and its shortest edge on the picture.
const FRONT: [f64; 2] = [100.0, 150.0];
const SHORTEST: f64 = 0.04;
/// Where a sun's points may lie, across and down (frame units): on the paper beside the frame and
/// clear of the frames round it, wherever the frame sits on the sheet (`sheets/layout.rs`: the
/// empty original's slot at its left, the gaps between rows and the sheet's edge).
const PAPER: [[f64; 2]; 2] = [[-1.0, 1.15], [-0.15, 1.25]];
/// The share of the shadows' outline the eye sees, at least: the rest hidden behind the boxes.
const SEEN: f64 = 0.5;

impl ShadowTask {
    /// Whether it keeps every rule: the horizon, the boxes, the boxes apart, the shadows, how much
    /// of them is seen, the sun's points, the givens inside and clear, the lamp inside.
    pub(crate) fn keeps(&self) -> bool {
        let inside = |p: &Point| p.iter().all(|v| (INSIDE..=1.0 - INSIDE).contains(v));
        let horizon = (HORIZON[0]..=HORIZON[1]).contains(&self.horizon());
        let boxes = self.boxes.iter().all(drawable);
        let outlines: Vec<Vec<Point>> = self.boxes.iter().map(outline_of).collect();
        let boxes_apart = apart(&outlines[0], &outlines[1]) >= APART;
        let shadows = self.shadows.keeps(LONG, APART) && self.shadows.points().all(|p| inside(&p));
        let length = |hidden: bool| -> f64 {
            let pieces = self.shadows.outline.iter().filter(|(_, h)| *h == hidden);
            pieces.map(|([a, b], _)| distance(*a, *b)).sum()
        };
        let seen = length(false) >= SEEN * (length(false) + length(true));
        let on_paper = |p: &Point| (0..2).all(|i| (PAPER[i][0]..=PAPER[i][1]).contains(&p[i]));
        let near_points = match (self.throw, self.points) {
            (Throw::Side, _) => true,
            (_, Some(points)) => points.iter().all(on_paper),
            _ => false,
        };
        let given = self.given_lines();
        let given_inside = given.iter().flatten().all(inside);
        let clear = given.iter().all(|line| {
            (outlines.iter().chain(&self.shadows.areas)).all(|area| apart(line, area) >= APART)
        });
        let lamp = !self.throw.lamp() || self.points.is_some_and(|p| p.iter().all(inside));
        let kept = [horizon, boxes, boxes_apart, shadows, seen, near_points];
        kept.into_iter().all(|kept| kept) && given_inside && clear && lamp
    }

    /// The light's given lines on the picture: the pin and its shadow, or the lamp's upright.
    pub(crate) fn given_lines(&self) -> Vec<Vec<Point>> {
        match (self.pinned, self.points) {
            (Some([foot, top, tip]), _) => vec![vec![foot, top], vec![foot, tip]],
            (None, Some([lamp, foot])) if self.throw.lamp() => vec![vec![foot, lamp]],
            _ => Vec::new(),
        }
    }
}

/// Whether a box of the scene can be drawn as the boxes kind's are: inside the frame, its vanishing
/// points within `FAR` of the frame's middle, its front angle V大's, no edge too short to draw.
/// (Two boxes and their shadows share the frame: each smaller than a box alone, its faces may be
/// narrower than a lone box's.)
fn drawable(seen: &PerspectiveBox) -> bool {
    let inside = |v: f64| (INSIDE..=1.0 - INSIDE).contains(&v);
    let corners = seen.corners.iter().all(|c| inside(c[0]) && inside(c[1]));
    let points =
        (seen.vanishing.iter().take(2)).all(|p| p.is_some_and(|p| distance(p, [0.5, 0.5]) <= FAR));
    let angle = (FRONT[0]..=FRONT[1]).contains(&seen.front_angle());
    let edges = (seen.edges().iter()).all(|(a, b, _)| distance(*a, *b) >= SHORTEST);
    corners && points && angle && edges
}

/// A box's outline on the picture: its corners' hull, closed.
fn outline_of(seen: &PerspectiveBox) -> Vec<Point> {
    let mut round = convex(seen.corners.to_vec());
    round.extend(round.first().copied());
    round
}
