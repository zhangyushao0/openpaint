//! A curved object in space (S154f, `design.rs`): its design built in its box as a view lays it
//! (`Lying`): each segment a frustum (a cylinder and a cone are frustums too) or a dome (objects'
//! `Ball`, taken on the outer side of its flat face only: its inner half lies in the wider end
//! it sits on, so the union is the same), a spout a frustum, a handle's bars objects' hulls. What a
//! ray meets first, from any point (`Parts::cast`; the main body alone where a part on its side
//! meets it), and whether a point lies inside a part serve the exact answer (`answer.rs`) and the
//! picture's depth test (`picture.rs`).
//!
//! The world: y up, z away from the eye, which looks level from the origin.

use crate::kinds::curved_rotations::attached::handle_bars;
use crate::kinds::curved_rotations::design::{Attachment, Design, SegmentKind, spout_root};
use crate::kinds::objects::object_body::{Ball, Hit, Hull, Placed, TOUCH};
use crate::kinds::objects::object_design::Local;
use crate::kinds::objects::object_rays::{ball_hit, hull_hit};
use crate::maths::vector::{Vector, add, dot, norm, sub, times, unit};

/// A frustum's surfaces are numbered from here (its side, its foot, its top), a spout's at
/// `SPOUT`; a handle's planes from `attached::BARS`, a dome's are `BALL_SURFACES` on.
const FRUSTUMS: u32 = 10;
const SPOUT: u32 = 900;

/// How a view lays the body in its box: its axis along the box's `axis` (0 its width towards the
/// right vanishing point, 1 up, 2 its depth towards the left one), from its foot `forward` or
/// back; its side `+x` along `side`, `out` or back. Never mirrored where it matters: the body and
/// what stands on its side are alike either side of the plane they share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Lying {
    pub(crate) axis: usize,
    pub(crate) forward: bool,
    pub(crate) side: usize,
    pub(crate) out: bool,
}

impl Lying {
    /// Whether it stands: its axis up, its foot at the bottom.
    pub(crate) const fn standing(self) -> bool {
        self.axis == 1
    }

    /// The box's axis across both the body's axis and its side.
    const fn third(self) -> usize {
        3 - self.axis - self.side
    }

    /// The box's own coordinates of the body's point `[s, x, y]`: `s` along its axis (a share of
    /// its length from its foot), `x` towards its side and `y` across (shares of its width from
    /// the axis).
    pub(crate) fn local(self, [s, x, y]: [f64; 3]) -> Local {
        let mut p = [0.0; 3];
        p[self.axis] = if self.forward { s } else { 1.0 - s };
        p[self.side] = 0.5 + if self.out { x } else { -x };
        p[self.third()] = 0.5 + y;
        p
    }

    /// The box's width, height and depth for a body `width` wide and `length` long.
    pub(crate) fn size(self, width: f64, length: f64) -> [f64; 3] {
        let mut size = [width; 3];
        size[self.axis] = length;
        size
    }
}

/// The body's own frame in the world: its foot's centre, its axis, side and across (one long),
/// its length and width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    pub(crate) foot: Vector,
    pub(crate) axis: Vector,
    pub(crate) side: Vector,
    pub(crate) across: Vector,
    pub(crate) length: f64,
    pub(crate) width: f64,
}

impl Frame {
    /// The frame `lying` gives the body in the box `placed`.
    pub(crate) fn of(lying: Lying, placed: Placed) -> Self {
        let at = |p: [f64; 3]| placed.world(lying.local(p));
        let foot = at([0.0; 3]);
        let [along, side, across] =
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|p| sub(at(p), foot));
        Self {
            foot,
            axis: unit(along),
            side: unit(side),
            across: unit(across),
            length: norm(along),
            width: norm(side),
        }
    }

    /// Where the body's point `[s, x, y]` lies in the world (`Lying::local`'s terms).
    pub(crate) fn at(&self, [s, x, y]: [f64; 3]) -> Vector {
        let along = times(self.axis, s * self.length);
        let out = add(
            times(self.side, x * self.width),
            times(self.across, y * self.width),
        );
        add(add(self.foot, along), out)
    }
}

/// A frustum: its foot's centre, its axis (one long) towards its top, how long it is, its foot's
/// and its top's radii (a cylinder's alike, a cone's point none).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frustum {
    pub(crate) centre: Vector,
    pub(crate) axis: Vector,
    pub(crate) length: f64,
    pub(crate) radii: [f64; 2],
}

impl Frustum {
    /// Its radius `s` along its axis from its foot.
    fn reach(&self, s: f64) -> f64 {
        self.radii[0] + (self.radii[1] - self.radii[0]) * s / self.length
    }

    /// How far `p` lies outside it (negative inside): past its foot, its top or its side (the
    /// side measured across the axis).
    pub(crate) fn off(&self, p: Vector) -> f64 {
        let from = sub(p, self.centre);
        let s = dot(from, self.axis);
        let radial = norm(sub(from, times(self.axis, s)));
        (-s).max(s - self.length).max(radial - self.reach(s))
    }

    /// Where the ray from `origin` along `d` enters it, its side `surface`, its foot the next and
    /// its top the one after.
    pub(crate) fn hit(&self, (origin, d): (Vector, Vector), surface: u32) -> Option<Hit> {
        let w = sub(origin, self.centre);
        let (wa, da) = (dot(w, self.axis), dot(d, self.axis));
        let h = self.length;
        let (low, high) = if da.abs() < 1e-15 {
            if !(0.0..=h).contains(&wa) {
                return None;
            }
            (f64::NEG_INFINITY, f64::INFINITY)
        } else {
            let [t0, t1] = [-wa / da, (h - wa) / da];
            (t0.min(t1), t0.max(t1))
        };
        let (wp, dp) = (sub(w, times(self.axis, wa)), sub(d, times(self.axis, da)));
        // Inside the side where |wp + t dp|² <= reach(t)², reach = g0 + g1 t.
        let k = (self.radii[1] - self.radii[0]) / h;
        let (g0, g1) = (self.radii[0] + k * wa, k * da);
        let quadratic = [
            dot(dp, dp) - g1 * g1,
            2.0 * (dot(wp, dp) - g0 * g1),
            dot(wp, wp) - g0 * g0,
        ];
        let inside = |t: f64| {
            let [a, b, c] = quadratic;
            a * t * t + b * t + c <= 0.0 && g0 + g1 * t >= 0.0
        };
        let (enter, leave) = interval(quadratic, (low, high), inside)?;
        (enter > 0.0 && enter <= leave).then(|| {
            let on_end = (enter - low).abs() <= 1e-12 * enter.abs().max(1.0);
            let (normal, surface) = if on_end && da > 0.0 {
                (times(self.axis, -1.0), surface + 1)
            } else if on_end {
                (self.axis, surface + 2)
            } else {
                (self.side_normal(add(origin, times(d, enter))), surface)
            };
            Hit {
                t: enter,
                normal,
                round: true,
                surface,
            }
        })
    }

    /// Its side's outward normal at `at`.
    fn side_normal(&self, at: Vector) -> Vector {
        let from = sub(at, self.centre);
        let out = unit(sub(from, times(self.axis, dot(from, self.axis))));
        let tilt = self.radii[0] - self.radii[1];
        unit(add(times(out, self.length), times(self.axis, tilt)))
    }
}

/// Where in `[low, high]` a ray is `inside` a frustum: the breaks the quadratic's roots make, each
/// piece tried at its middle (the frustum is convex: the pieces inside run together).
fn interval(
    [a, b, c]: [f64; 3],
    (low, high): (f64, f64),
    inside: impl Fn(f64) -> bool,
) -> Option<(f64, f64)> {
    let mut breaks = vec![low, high];
    if a.abs() > 1e-15 {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let root = disc.sqrt();
            breaks.extend([(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)]);
        }
    } else if b.abs() > 1e-15 {
        breaks.push(-c / b);
    }
    breaks.retain(|t| t.is_finite() && *t >= low && *t <= high);
    breaks.sort_by(f64::total_cmp);
    let mut held: Option<(f64, f64)> = None;
    for pair in breaks.windows(2) {
        if pair[1] > pair[0] && inside((pair[0] + pair[1]) / 2.0) {
            held = Some(held.map_or((pair[0], pair[1]), |(from, _)| (from, pair[1])));
        }
    }
    held
}

/// A dome: its ball, and the way from its flat face's centre (the ball's) to its top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Dome {
    pub(crate) ball: Ball,
    pub(crate) out: Vector,
}

impl Dome {
    /// Whether `p` lies on the dome's side of its flat face (within `TOUCH`).
    pub(crate) fn outer(&self, p: Vector) -> bool {
        dot(sub(p, self.ball.centre), self.out) >= -TOUCH
    }

    fn off(&self, p: Vector) -> f64 {
        let from = sub(p, self.ball.centre);
        (norm(from) - self.ball.radius).max(-dot(from, self.out))
    }

    /// Where the ray enters it, as its ball's hit where that lies on its outer side: a ray
    /// entering the ball's inner half reaches the dome only through its flat face, which the end
    /// under it covers, so that end meets it first.
    fn hit(&self, (origin, d): (Vector, Vector), index: usize) -> Option<Hit> {
        let hit = ball_hit(&self.ball, (origin, d), index)?;
        self.outer(add(origin, times(d, hit.t))).then_some(hit)
    }
}

/// A segment of the body in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Part {
    Frustum(Frustum),
    Dome(Dome),
}

/// A curved object in the world: its segments from its foot, a spout, a handle's bars.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parts {
    pub(crate) frame: Frame,
    pub(crate) segments: Vec<Part>,
    pub(crate) spout: Option<Frustum>,
    pub(crate) bars: Vec<Hull>,
}

impl Parts {
    /// `design` built in the box `placed` as `lying` lays it.
    pub(crate) fn of(design: &Design, lying: Lying, placed: Placed) -> Self {
        let frame = Frame::of(lying, placed);
        let last = design.segments.len() - 1;
        let segments = (design.segments.iter().enumerate())
            .map(|(k, segment)| {
                let radii = segment.wide.map(|w| w * frame.width / 2.0);
                if segment.shape == SegmentKind::Dome {
                    let (flat, out) = if k == last {
                        (segment.along[0], frame.axis)
                    } else {
                        (segment.along[1], times(frame.axis, -1.0))
                    };
                    let centre = frame.at([flat, 0.0, 0.0]);
                    let ball = Ball {
                        centre,
                        radius: radii[0],
                    };
                    return Part::Dome(Dome { ball, out });
                }
                Part::Frustum(Frustum {
                    centre: frame.at([segment.along[0], 0.0, 0.0]),
                    axis: frame.axis,
                    length: (segment.along[1] - segment.along[0]) * frame.length,
                    radii,
                })
            })
            .collect();
        let (spout, bars) = match design.attachment {
            Some(Attachment::Spout { along, wide, reach }) => {
                let tip = wide[1] / 2.0 * frame.width;
                let root = spout_root(wide[0], reach) * frame.width;
                let spout = Frustum {
                    centre: frame.at([along, 0.0, 0.0]),
                    axis: frame.side,
                    length: (0.5 + reach) * frame.width,
                    radii: [root, tip],
                };
                (Some(spout), Vec::new())
            }
            Some(Attachment::Handle { along, reach }) => (None, handle_bars(&frame, along, reach)),
            None => (None, Vec::new()),
        };
        Self {
            frame,
            segments,
            spout,
            bars,
        }
    }

    /// What the ray from `origin` along `d` meets first, `t` 1 at `origin + d`; of the main body
    /// alone where `main`.
    pub(crate) fn cast(&self, (origin, d): (Vector, Vector), main: bool) -> Option<Hit> {
        let ray = (origin, d);
        let segments = (self.segments.iter().enumerate()).filter_map(|(k, part)| match part {
            Part::Frustum(frustum) => frustum.hit(ray, FRUSTUMS + 4 * k as u32),
            Part::Dome(dome) => dome.hit(ray, k),
        });
        let spout = (self.spout.iter()).filter_map(|spout| spout.hit(ray, SPOUT));
        let bars = (self.bars.iter()).filter_map(|bar| hull_hit(bar, origin, d));
        let all: Vec<Hit> = if main {
            segments.collect()
        } else {
            segments.chain(spout).chain(bars).collect()
        };
        all.into_iter().min_by(|a, b| a.t.total_cmp(&b.t))
    }

    /// What the eye's ray towards `p` meets first, `t` 1 at `p`.
    pub(crate) fn hit(&self, p: Vector) -> Option<Hit> {
        self.cast(([0.0; 3], p), false)
    }

    /// Whether `p` lies inside a part, beyond `TOUCH` from its surfaces.
    pub(crate) fn inside(&self, p: Vector) -> bool {
        let segments = self.segments.iter().any(|part| match part {
            Part::Frustum(frustum) => frustum.off(p) < -TOUCH,
            Part::Dome(dome) => dome.off(p) < -TOUCH,
        });
        let spout = self.spout.iter().any(|spout| spout.off(p) < -TOUCH);
        let bars = (self.bars.iter()).any(|bar| {
            (bar.planes.iter()).all(|plane| dot(plane.normal, p) - plane.offset < -TOUCH)
        });
        segments || spout || bars
    }
}
