//! Cast shadows' answer (S153): the outline of the shadows the boxes throw, exact. A box's shadow
//! on the ground is the outline round its base corners and its top corners' shadows
//! (`light::box_shadow`); its sides from a base corner to a shadow corner are its uprights'
//! shadows, from shadow corner to shadow corner its top edges'. Where two boxes' shadows overlap
//! the shadow is one: each outline is cut where it runs inside the other. Where the nearer box's
//! shadow climbs the farther's side (稳), the side holds the rest (`climb.rs`). Each piece is split
//! where the boxes hide it from the eye (drawn dashed).
//!
//! Each corner of a shadow is constructed as a perspective drawing finds it: the ray from the
//! light (its point, or along the pin's slant) through a top corner meets the ground line from the
//! shadows' point (the horizon's point under the sun, the lamp's foot, or along the pin's shadow)
//! through the corner under it.

use super::climb::climbed;
use crate::geometry::light::{Light, box_shadow};
use crate::kinds::objects::object_body::{Body, Placed};
use crate::kinds::objects::object_design::Block;
use crate::kinds::objects::object_hull::hull;
use crate::kinds::solids::solid_answer::View;
use crate::maths::path::{Point, apart, distance};
use crate::maths::vector::{Vector, add, sub, times};

/// Steps along a piece looked at for what hides it, and halvings that find where that changes.
const STEPS: usize = 48;
const HALVINGS: usize = 30;
/// A point is seen when nothing lies nearer on its ray than this share of the way.
const SEEN: f64 = 1e-6;

/// A shadow corner on the picture: the top corner whose shadow it is, the corner under that one,
/// and the shadow itself (frame units); on a side it climbs (稳), where the ground line through the
/// foot meets the side, the shadow straight above it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corner {
    pub top: Point,
    pub foot: Point,
    pub shadow: Point,
    pub upright: Option<Point>,
}

/// The boxes' shadows: the outline's pieces (frame units) with whether the boxes hide them, the
/// shadow corners on the ground, the shadows on the picture (each closed), each top corner's reach,
/// whether they keep apart or climb as the scene asks.
#[derive(Debug, Clone, PartialEq)]
pub struct Shadows {
    pub outline: Vec<([Point; 2], bool)>,
    pub corners: Vec<Corner>,
    pub(crate) areas: Vec<Vec<Point>>,
    pub(crate) climbing: Option<bool>,
}

/// The shadows the boxes of `corners` throw under `light` on the ground at `ground`, as `view` sees
/// them; the first climbing the second's side where `climbing`.
pub(crate) fn shadows(
    corners: &[[Vector; 8]],
    (light, ground, climbing): (&Light, f64, bool),
    view: View,
) -> Shadows {
    let hulls: Vec<Vec<(Vector, usize)>> = (corners.iter())
        .map(|box_corners| box_shadow(light, box_corners, ground).unwrap_or_default())
        .collect();
    let (mut pieces, mut found) = on_ground(corners, &hulls, view);
    let climb = climbing.then(|| climbed(corners, light)).flatten();
    if let Some(climb) = &climb {
        pieces.extend(climb.pieces.iter().copied());
        found.extend(
            (climb.corners.iter()).map(|[top, foot, upright, shadow]| Corner {
                top: view.at(*top),
                foot: view.at(*foot),
                shadow: view.at(*shadow),
                upright: Some(view.at(*upright)),
            }),
        );
    }
    let body = boxes(corners);
    let outline = (pieces.into_iter())
        .flat_map(|piece| seen_pieces(&body, piece))
        .map(|([a, b], hidden)| ([view.at(a), view.at(b)], hidden))
        .collect();
    let areas = (hulls.iter())
        .map(|outline| {
            let mut area: Vec<Point> = outline.iter().map(|(p, _)| view.at(*p)).collect();
            area.extend(area.first().copied());
            area
        })
        .collect();
    Shadows {
        outline,
        corners: found,
        areas,
        climbing: climbing.then(|| climb.is_some_and(|climb| climb.kept)),
    }
}

/// The pieces of the shadows' outline on the ground (the boxes of `corners` throwing the outlines
/// `hulls`), each cut where it runs inside another box's shadow, and the corners found there as
/// `view` sees them.
fn on_ground(
    corners: &[[Vector; 8]],
    hulls: &[Vec<(Vector, usize)>],
    view: View,
) -> (Vec<[Vector; 2]>, Vec<Corner>) {
    let mut pieces: Vec<[Vector; 2]> = Vec::new();
    let mut found = Vec::new();
    for (i, outline) in hulls.iter().enumerate() {
        let others = || {
            (hulls.iter().enumerate())
                .filter(|(j, _)| *j != i)
                .map(|(_, o)| o)
        };
        for k in 0..outline.len() {
            let ((p, from), (q, to)) = (outline[k], outline[(k + 1) % outline.len()]);
            if from & 2 == 0 && to & 2 == 0 {
                // A base edge: the box's own.
                continue;
            }
            let mut kept = vec![[p, q]];
            for other in others() {
                kept = (kept.into_iter())
                    .flat_map(|piece| outside(piece, other))
                    .collect();
            }
            pieces.extend(kept);
            // A top corner's shadow on the ground, unless it falls in another shadow.
            let shown = others().all(|other| outside([p, p], other).len() == 1);
            if from & 2 != 0 && shown {
                let [top, foot] = [corners[i][from], corners[i][from & !2]].map(|c| view.at(c));
                let shadow = view.at(p);
                let upright = None;
                found.push(Corner {
                    top,
                    foot,
                    shadow,
                    upright,
                });
            }
        }
    }
    (pieces, found)
}

/// The boxes of `corners` as one body, for what they hide.
fn boxes(corners: &[[Vector; 8]]) -> Body {
    let placed = |c: &[Vector; 8]| Placed {
        origin: c[0],
        axes: [sub(c[1], c[0]), sub(c[2], c[0]), sub(c[4], c[0])],
    };
    let hulls = (corners.iter().enumerate())
        .filter_map(|(k, c)| {
            let mut found = hull(&Block::WHOLE, &[], placed(c))?;
            // Each plane a surface of its own.
            let first = (k * found.planes.len()) as u32;
            found.surfaces = (0..found.planes.len()).map(|p| first + p as u32).collect();
            Some(found)
        })
        .collect();
    Body {
        placed: placed(&corners[0]),
        hulls,
        rounds: Vec::new(),
        balls: Vec::new(),
    }
}

/// The parts of `piece` (on the ground) outside the shadow `outline` (in order round on the
/// ground, x and z): the whole where it runs outside it.
fn outside([p, q]: [Vector; 2], outline: &[(Vector, usize)]) -> Vec<[Vector; 2]> {
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    let along = sub(q, p);
    let side = |o: Vector, e: Vector, v: Vector| e[0] * (v[2] - o[2]) - e[2] * (v[0] - o[0]);
    for k in 0..outline.len() {
        let (o, next) = (outline[k].0, outline[(k + 1) % outline.len()].0);
        let e = sub(next, o);
        // Inside where `side` is positive (the outline runs round anticlockwise in x and z).
        let at_p = side(o, e, p);
        let slope = e[0] * along[2] - e[2] * along[0];
        if slope.abs() < 1e-15 {
            if at_p <= 1e-12 {
                return vec![[p, q]];
            }
            continue;
        }
        let t = -at_p / slope;
        if slope > 0.0 {
            enter = enter.max(t);
        } else {
            leave = leave.min(t);
        }
    }
    if enter >= leave - 1e-12 {
        return vec![[p, q]];
    }
    let at = |t: f64| add(p, times(along, t));
    [[0.0, enter], [leave, 1.0]]
        .into_iter()
        .filter(|[a, b]| b - a > 1e-9)
        .map(|[a, b]| [at(a), at(b)])
        .collect()
}

/// `piece` split where `body` hides it from the eye, each part with whether it is hidden.
fn seen_pieces(body: &Body, [a, b]: [Vector; 2]) -> Vec<([Vector; 2], bool)> {
    let at = |t: f64| add(a, times(sub(b, a), t));
    let hidden = |t: f64| body.hit(at(t)).is_some_and(|hit| hit.t < 1.0 - SEEN);
    let mut parts: Vec<([Vector; 2], bool)> = Vec::new();
    let mut from = 0.0;
    let mut was = hidden(0.5 / STEPS as f64);
    for k in 1..STEPS {
        let t = (k as f64 + 0.5) / STEPS as f64;
        let now = hidden(t);
        if now != was {
            let (mut low, mut high) = (t - 1.0 / STEPS as f64, t);
            for _ in 0..HALVINGS {
                let middle = (low + high) / 2.0;
                if hidden(middle) == was {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            let cut = (low + high) / 2.0;
            parts.push(([at(from), at(cut)], was));
            (from, was) = (cut, now);
        }
    }
    parts.push(([at(from), b], was));
    parts
}

/// The points of `points` round their outline, anticlockwise (x right, y up), those inside left
/// out.
pub(crate) fn convex(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let turn = |o: Point, a: Point, b: Point| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut round: Vec<Point> = Vec::new();
    for pass in [false, true] {
        let start = round.len();
        let walk: Vec<Point> = if pass {
            points.iter().rev().copied().collect()
        } else {
            points.clone()
        };
        for p in walk {
            while round.len() >= start + 2
                && turn(round[round.len() - 2], round[round.len() - 1], p) <= 1e-12
            {
                round.pop();
            }
            round.push(p);
        }
        round.pop();
    }
    round
}

impl Shadows {
    /// Whether each top corner's shadow lies at least `long` from its corner, and the shadows keep
    /// apart by `gap` (on the picture) or climb as the scene asks.
    pub(crate) fn keeps(&self, long: f64, gap: f64) -> bool {
        let reach = (self.corners.iter()).all(|c| distance(c.foot, c.shadow) >= long);
        let kept = match self.climbing {
            Some(climbs) => climbs,
            None => (self.areas.len() < 2) || apart(&self.areas[0], &self.areas[1]) >= gap,
        };
        reach && kept
    }

    /// Every point of the outline.
    pub(crate) fn points(&self) -> impl Iterator<Item = Point> + '_ {
        (self.outline.iter()).flat_map(|(ends, _)| ends.iter().copied())
    }

    /// How each shadow corner is found, given the light's two points (`points`: the light's and
    /// the shadows', or the lamp and its foot; none for a sun parallel to the picture): the ray
    /// through its top corner and the ground line through its foot, which meet there; on a side it
    /// climbs, the ground line to the side and the upright from there (frame units).
    #[must_use]
    pub fn construction(&self, points: Option<[Point; 2]>) -> Vec<[Point; 2]> {
        let mut lines = Vec::new();
        for corner in &self.corners {
            let [ray, ground] = points.unwrap_or([corner.top, corner.foot]);
            lines.push([ray, corner.shadow]);
            match corner.upright {
                Some(upright) => lines.extend([[ground, upright], [upright, corner.shadow]]),
                None => lines.push([ground, corner.shadow]),
            }
        }
        lines
    }
}
