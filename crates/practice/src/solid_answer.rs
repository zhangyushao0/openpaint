//! A solid's exact answer (S93, `solid.rs`): the solid in space projected through the eye that
//! sees its box (`space.rs`'s, looking level), never one found on the picture.
//!
//! - An end's or a base's circle, inscribed in its square face, is seen as the ellipse the face's
//!   homography carries (`ellipse.rs`), touching the face's edges at their middles.
//! - A cylinder's sides are the lines along it whose planes through the eye touch it: where the
//!   rim's outward direction `n` holds `n · c + r = 0` (`c` an end's centre, `r` its radius), the
//!   same on both ends, so each side touches both ellipses. An end whose face the eye sees shows
//!   whole; the other's rim (both, where the eye lies between their planes) is hidden where the
//!   side surface turns from the eye (`n · c + r > 0`).
//! - A cone's sides run from its point and touch its base: where the surface's normal there,
//!   `h n + r a` (`a` its axis, `h` its height), lies in a plane through the eye. Its base's rim is
//!   hidden where that surface turns away, unless the base itself faces the eye.
//! - A sphere's outline is where the eye's lines of sight touch it: a cone of sight cut by the
//!   picture, the conic `(d · C)² = |d|² (|C|² - r²)` of each line of sight `d`, an ellipse.
//!
//! The world: y up, z away from the eye. Frame units on the picture.

use crate::ellipse::{Ellipse, EllipseGuide};
use crate::ellipse_fit::placed;
use crate::path::Point;
use crate::space::projected;

/// An outline drawn in this many pieces.
const PIECES: usize = 192;
/// A side's touches lie well clear of where they would meet and vanish: the cosine that finds them
/// at most this (no cone's point nearly inside its base, no cylinder seen nearly down its axis).
const TOUCHING: f64 = 0.95;

pub(crate) type Vector = [f64; 3];

pub(crate) fn add(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| a[k] + b[k])
}

pub(crate) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| a[k] - b[k])
}

fn times(a: Vector, by: f64) -> Vector {
    a.map(|v| v * by)
}

fn dot(a: Vector, b: Vector) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}

/// A square face in the world: a corner and its two edges from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Square {
    pub(crate) corner: Vector,
    pub(crate) u: Vector,
    pub(crate) v: Vector,
}

impl Square {
    pub(crate) fn centre(&self) -> Vector {
        add(self.corner, times(add(self.u, self.v), 0.5))
    }

    /// Its inscribed circle's point at `t` (radians) from its centre along `u`.
    pub(crate) fn rim(&self, t: f64) -> Vector {
        let out = add(times(self.u, 0.5 * t.cos()), times(self.v, 0.5 * t.sin()));
        add(self.centre(), out)
    }

    /// Its corners in order round: the corner, along `u`, opposite, along `v`.
    pub(crate) fn corners(&self) -> [Vector; 4] {
        let c = self.corner;
        [
            c,
            add(c, self.u),
            add(add(c, self.u), self.v),
            add(c, self.v),
        ]
    }

    /// Its circle's radius squared.
    fn radius2(&self) -> f64 {
        dot(self.u, self.u) / 4.0
    }

    /// `(a, b)` of `a cos t + b sin t`: the outward rim `rim(t) - centre` against `towards`.
    fn against(&self, towards: Vector) -> (f64, f64) {
        (dot(self.u, towards) / 2.0, dot(self.v, towards) / 2.0)
    }
}

/// How the eye's image lies in the frame: frame units per image unit and where its origin lies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct View {
    pub(crate) scale: f64,
    pub(crate) offset: Point,
}

impl View {
    /// Where the world point `p` lies in the frame.
    pub(crate) fn at(&self, p: Vector) -> Point {
        let [x, y] = projected(p);
        [
            x * self.scale + self.offset[0],
            y * self.scale + self.offset[1],
        ]
    }
}

/// The solid in space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Body {
    /// A cylinder between its ends, the one whose face the eye sees first.
    Cylinder([Square; 2]),
    /// A cone on its base, its point.
    Cone(Square, Vector),
    /// A sphere: its centre and radius.
    Sphere(Vector, f64),
}

/// A stretch of an outline: seen, or hidden behind the solid (dashed).
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    pub points: Vec<Point>,
    pub hidden: bool,
}

/// A solid as the eye sees it, in frame units.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidAnswer {
    /// A cylinder's two ends (the one seen whole first), a cone's base, a sphere's outline.
    pub ellipses: Vec<Ellipse>,
    /// Each ellipse in stretches, seen or hidden.
    pub outlines: Vec<Vec<Stretch>>,
    /// The sides: a cylinder's from one end to the other, a cone's from its point to its base;
    /// none where they cannot be drawn.
    pub sides: Vec<[Point; 2]>,
}

/// `body` seen through `view`; a cylinder's or a cone's ellipses those of `guides`.
pub(crate) fn answer(body: &Body, view: View, guides: &[EllipseGuide]) -> SolidAnswer {
    let exact = |k: usize| guides.get(k).map(|guide| guide.exact);
    match *body {
        Body::Cylinder([near, far]) => {
            let c = near.centre();
            let (a, b) = near.against(c);
            // The side surface turns from the eye where `n · c + r > 0` (times r); an end whose
            // face the eye sees shows whole, the other's rim (both, the eye between their
            // planes) hidden there.
            let turned = |t: f64| a * t.cos() + b * t.sin() + near.radius2() > 0.0;
            let axis = sub(far.centre(), c);
            let seen = [dot(axis, c) > 0.0, dot(axis, far.centre()) < 0.0];
            let sides = touches(a, b, near.radius2()).map(|ts| {
                ts.map(|t| [view.at(near.rim(t)), view.at(far.rim(t))])
                    .to_vec()
            });
            SolidAnswer {
                ellipses: [exact(0), exact(1)].into_iter().flatten().collect(),
                outlines: vec![
                    rim(&near, view, |t| !seen[0] && turned(t)),
                    rim(&far, view, |t| !seen[1] && turned(t)),
                ],
                sides: sides.unwrap_or_default(),
            }
        }
        Body::Cone(base, point) => {
            let c = base.centre();
            let axis = sub(point, c);
            let height = dot(axis, axis).sqrt().max(f64::MIN_POSITIVE);
            let along = dot(axis, c) / height;
            let (a, b) = base.against(c);
            let (a, b) = (height * a, height * b);
            let r2 = base.radius2();
            let constant = height * r2 + r2 * along;
            // The base faces the eye where its outward side, against the axis, points to it.
            let base_seen = along > 0.0;
            let turned = |t: f64| !base_seen && a * t.cos() + b * t.sin() + constant > 0.0;
            let sides = touches(a, b, constant)
                .map(|ts| ts.map(|t| [view.at(point), view.at(base.rim(t))]).to_vec());
            SolidAnswer {
                ellipses: exact(0).into_iter().collect(),
                outlines: vec![rim(&base, view, turned)],
                sides: sides.unwrap_or_default(),
            }
        }
        Body::Sphere(centre, radius) => {
            let outline = sphere_outline(centre, radius, view);
            SolidAnswer {
                ellipses: vec![outline],
                outlines: vec![vec![Stretch {
                    points: outline.outline(PIECES),
                    hidden: false,
                }]],
                sides: Vec::new(),
            }
        }
    }
}

/// Where `a cos t + b sin t + c` is 0: the two touches, well apart (`TOUCHING`); none else.
fn touches(a: f64, b: f64, c: f64) -> Option<[f64; 2]> {
    let r = a.hypot(b);
    let k = -c / r.max(f64::MIN_POSITIVE);
    (k.abs() <= TOUCHING).then(|| {
        let (phi, d) = (b.atan2(a), k.acos());
        [phi - d, phi + d]
    })
}

/// `square`'s circle seen through `view` in stretches, hidden where `turned`.
fn rim(square: &Square, view: View, turned: impl Fn(f64) -> bool) -> Vec<Stretch> {
    let mut out: Vec<Stretch> = Vec::new();
    for k in 0..=PIECES {
        let t = k as f64 / PIECES as f64 * std::f64::consts::TAU;
        let (p, hidden) = (view.at(square.rim(t)), turned(t));
        match out.last_mut() {
            Some(last) if last.hidden == hidden => last.points.push(p),
            _ => {
                // A new stretch begins where the last ended, so the outline stays whole.
                let joined = out.last().and_then(|last| last.points.last().copied());
                let points = joined.into_iter().chain([p]).collect();
                out.push(Stretch { points, hidden });
            }
        }
    }
    // Round the start: the last stretch runs on into the first when alike.
    if out.len() > 2 && out[0].hidden == out[out.len() - 1].hidden {
        let first = out.remove(0);
        if let Some(last) = out.last_mut() {
            last.points.extend(first.points.into_iter().skip(1));
        }
    }
    out
}

/// A sphere's outline: its cone of sight cut by the picture, seen through `view`; where it is no
/// ellipse (never for a sphere well in front of the eye), the circle about its centre's image.
fn sphere_outline(centre: Vector, radius: f64, view: View) -> Ellipse {
    let [cx, cy, cz] = centre;
    let k = dot(centre, centre) - radius * radius;
    // Lines of sight (x, -y, 1) to the image's (x, y).
    let image = [
        cx * cx - k,
        -2.0 * cx * cy,
        cy * cy - k,
        2.0 * cx * cz,
        -2.0 * cy * cz,
        cz * cz - k,
    ];
    let fallback = Ellipse {
        centre: view.at(centre),
        axes: [radius / cz.max(f64::MIN_POSITIVE) * view.scale; 2],
        angle: 0.0,
    };
    Ellipse::of_conic(placed(image, view.offset, view.scale)).unwrap_or(fallback)
}
