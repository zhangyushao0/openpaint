//! An object's exact answer (S94, `object.rs`): its edges and outlines as the eye sees them,
//! hidden by its own parts, never found on a picture. Every line that may show is a hull's edge
//! (where two of its planes meet), a round part's end circle, or a round part's outline along its
//! side (where planes through the eye touch it, as S93's sides). Along each, the stretches that
//! are edges of the object (`crease`: its surface turns there, not where two blocks of it only
//! meet flat) and, of those, the ones the eye sees (`seen`: nothing nearer on the ray to them) are
//! found at even steps and their ends to the last bit by halving; then projected. Straight edges
//! on one line join (`merged`), so an edge two blocks share is one.
//!
//! The ends of an edge's stretches are the object's corners: where its surface turns another way.

use crate::geometry::ellipse::Ellipse;
use crate::geometry::ellipse::{EllipseGuide, square_on};
use crate::kinds::objects::object_body::{Body, TOUCH, Touch};
use crate::kinds::objects::object_design::Local;
use crate::kinds::solids::solid_answer::{SolidAnswer, Square, Stretch, View};
use crate::maths::path::{Point, distance};
use crate::maths::vector::{Vector, across, add, dot, norm, sub, times, unit};

/// Each round part's ends: their ellipses, their stretches, their squares as guides.
type Rims = (Vec<Ellipse>, Vec<Vec<Stretch>>, Vec<EllipseGuide>);

/// Steps along a line, frame units (a pixel and a half of a 400-pixel frame), and at least.
const STEP: f64 = 0.004;
const LEAST_STEPS: usize = 8;
/// Halvings that find where a stretch ends.
const HALVINGS: usize = 48;
/// The ring of points round a line that tells an edge from a flat seam: how many, how far (world
/// units, far beyond `TOUCH`, far inside any part's thickness).
const RING: usize = 48;
const RING_RADIUS: f64 = 1e-5;
/// A point is seen when nothing lies nearer on its ray than this share of the way.
const SEEN: f64 = 1e-6;
/// Lines within this of each other (frame units) are one.
const ONE_LINE: f64 = 1e-8;

/// An object as the eye sees it: the answer, the corners at the ends of the edges it shows (the
/// box's own coordinates), each round part's ends as squares with their ellipses, and its straight
/// edges and side outlines where its own parts hide them (S113), each a pixel and a half long at
/// least.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Seen {
    pub(crate) answer: SolidAnswer,
    pub(crate) corners: Vec<Local>,
    pub(crate) guides: Vec<EllipseGuide>,
    pub(crate) hidden: Vec<[Point; 2]>,
}

/// A line of the object that may show.
#[derive(Debug, Clone, Copy)]
enum Line {
    /// A hull's edge, its two ends.
    Edge([Vector; 2]),
    /// A round part's outline along its side, its two ends; the part.
    Outline([Vector; 2], usize),
    /// A round part's end circle, in its square.
    Rim(Square),
}

impl Line {
    fn at(&self, t: f64) -> Vector {
        match self {
            Self::Edge([a, b]) | Self::Outline([a, b], _) => add(*a, times(sub(*b, *a), t)),
            Self::Rim(square) => square.rim(t),
        }
    }

    fn along(&self, t: f64) -> Vector {
        match self {
            Self::Edge([a, b]) | Self::Outline([a, b], _) => sub(*b, *a),
            Self::Rim(square) => square.rim_along(t),
        }
    }

    /// Its parameter's span.
    const fn span(&self) -> f64 {
        match self {
            Self::Edge(_) | Self::Outline(..) => 1.0,
            Self::Rim(_) => std::f64::consts::TAU,
        }
    }

    /// Whether the object's surface turns at its point `t` (an outline: lies on the surface).
    fn shows(&self, body: &Body, t: f64) -> bool {
        let p = self.at(t);
        match self {
            Self::Outline(_, round) => !body.holds(p, -TOUCH, Some(*round)),
            Self::Edge(_) | Self::Rim(_) => crease(body, p, self.along(t)),
        }
    }
}

/// `body` as the eye sees it through `view`.
pub(crate) fn seen(body: &Body, view: View) -> Seen {
    let mut straight: Vec<[Point; 2]> = Vec::new();
    let mut hidden: Vec<[Point; 2]> = Vec::new();
    let mut corners: Vec<Local> = Vec::new();
    let steps = |line: &Line| {
        let long = (0..8)
            .map(|k| {
                let [a, b] = [k, k + 1].map(|i| line.at(line.span() * f64::from(i) / 8.0));
                distance(view.at(a), view.at(b))
            })
            .sum::<f64>();
        ((long / STEP) as usize).max(LEAST_STEPS)
    };
    for line in lines(body) {
        let creased = runs(steps(&line), (0.0, line.span()), |t| line.shows(body, t));
        for [from, to] in creased {
            let shown = runs(steps(&line), (from, to), |t| visible(body, line.at(t)));
            hidden.extend(
                gaps(&shown, [from, to])
                    .map(|[a, b]| [a, b].map(|t| view.at(line.at(t))))
                    .filter(|[a, b]| distance(*a, *b) >= STEP),
            );
            if shown.is_empty() {
                continue;
            }
            if matches!(line, Line::Edge(_)) {
                corners.extend([from, to].map(|t| body.placed.local(line.at(t))));
            }
            straight.extend(
                shown
                    .iter()
                    .map(|[a, b]| [a, b].map(|t| view.at(line.at(*t)))),
            );
        }
    }
    let (ellipses, outlines, guides) = rims(body, view);
    for round in &body.rounds {
        for square in [round.base, round.top()].iter().filter(|s| norm(s.u) > 0.0) {
            corners.extend(square.corners().map(|p| body.placed.local(p)));
        }
    }
    Seen {
        answer: SolidAnswer {
            ellipses,
            outlines,
            sides: merged(straight),
            whole: false,
        },
        corners: distinct(corners),
        guides,
        hidden: merged(hidden),
    }
}

/// The stretches of `[from, to]` between the `shown` runs (in order, within it).
fn gaps(shown: &[[f64; 2]], [from, to]: [f64; 2]) -> impl Iterator<Item = [f64; 2]> {
    let starts = std::iter::once(from).chain(shown.iter().map(|run| run[1]));
    let ends = shown.iter().map(|run| run[0]).chain(std::iter::once(to));
    starts.zip(ends).filter(|(a, b)| b > a).map(|(a, b)| [a, b])
}

/// Every line of `body` that may show: its hulls' edges and its round parts' outlines.
fn lines(body: &Body) -> Vec<Line> {
    let mut lines: Vec<Line> = (body.hulls.iter())
        .flat_map(|hull| hull.edges.iter())
        .map(|edge| Line::Edge(edge.map(|p| body.placed.world(p))))
        .collect();
    for (index, round) in body.rounds.iter().enumerate() {
        lines.extend(outlines(round).map(|ends| Line::Outline(ends, index)));
    }
    lines
}

/// A round part's outlines along its side, where planes through the eye touch it (S93's sides).
fn outlines(
    round: &crate::kinds::objects::object_body::Round,
) -> impl Iterator<Item = [Vector; 2]> {
    let centre = round.base.centre();
    let [u, v] = across(round.axis);
    let r = round.radius();
    // A cylinder's rim point `c + r n` lies on an outline where `n · c + r = 0`; a cone's where
    // `h n · c + r (h + a · c) = 0`.
    let wanted = if round.cone {
        -r * (round.length + dot(round.axis, centre)) / round.length
    } else {
        -r
    };
    let (a, b) = (dot(u, centre), dot(v, centre));
    let reach = a.hypot(b).max(f64::MIN_POSITIVE);
    let touching = (wanted / reach).abs() <= 1.0;
    let (phi, spread) = (b.atan2(a), (wanted / reach).clamp(-1.0, 1.0).acos());
    let top = add(centre, times(round.axis, round.length));
    let line = move |t: f64| {
        let out = add(times(u, r * t.cos()), times(v, r * t.sin()));
        let foot = add(centre, out);
        let end = if round.cone {
            top
        } else {
            add(foot, times(round.axis, round.length))
        };
        [foot, end]
    };
    [phi - spread, phi + spread]
        .into_iter()
        .filter(move |_| touching)
        .map(line)
}

/// Each round part's end circles: their exact ellipses (each its square's, as S93's), their
/// stretches seen and hidden, and the squares as guides.
fn rims(body: &Body, view: View) -> Rims {
    let (mut ellipses, mut outlines, mut guides) = (Vec::new(), Vec::new(), Vec::new());
    for round in &body.rounds {
        for square in [round.base, round.top()] {
            if norm(square.u) <= 0.0 {
                continue;
            }
            let guide = square_on(square.corners().map(|p| view.at(p)));
            let line = Line::Rim(square);
            let tau = std::f64::consts::TAU;
            let shown = runs(240, (0.0, tau), |t| {
                line.shows(body, t) && visible(body, line.at(t))
            });
            outlines.push(stretches(&shown, |t| view.at(line.at(t))));
            ellipses.push(guide.exact);
            guides.push(guide);
        }
    }
    (ellipses, outlines, guides)
}

/// A circle's stretches from its `shown` runs (round from 0 to a full turn): seen and hidden in
/// turn, each a line through `at` of its parameter.
fn stretches(shown: &[[f64; 2]], at: impl Fn(f64) -> Point) -> Vec<Stretch> {
    let tau = std::f64::consts::TAU;
    let mut bounds: Vec<(f64, bool)> = vec![(0.0, false)];
    for [from, to] in shown {
        bounds.push((*from, true));
        bounds.push((*to, false));
    }
    bounds.push((tau, false));
    let mut out: Vec<Stretch> = Vec::new();
    for pair in bounds.windows(2) {
        let ((from, seen), (to, _)) = (pair[0], pair[1]);
        if to - from <= 1e-12 {
            continue;
        }
        let pieces = ((to - from) / tau * 192.0).ceil().max(1.0) as usize;
        let points = (0..=pieces)
            .map(|k| at(from + (to - from) * k as f64 / pieces as f64))
            .collect();
        out.push(Stretch {
            points,
            hidden: !seen,
        });
    }
    out
}

/// Whether the object's surface turns at `p` on a line heading `along`: not inside a part nor
/// off it, and not flat across it (two blocks meeting in one plane): one part's two surfaces
/// meet there, or the parts there fill a ring round it other than half, as a half-plane would.
fn crease(body: &Body, p: Vector, along: Vector) -> bool {
    let parts = match body.touching(p) {
        Touch::Inside => return false,
        Touch::Parts(parts) => parts,
    };
    match parts.as_slice() {
        [] => false,
        [surfaces] => *surfaces >= 2,
        _ => {
            let [e1, e2] = across(unit(along));
            let held: Vec<bool> = (0..RING)
                .map(|k| {
                    let turn = (k as f64 + 0.5) / RING as f64 * std::f64::consts::TAU;
                    let off = add(times(e1, turn.cos()), times(e2, turn.sin()));
                    body.holds(add(p, times(off, RING_RADIUS)), 0.0, None)
                })
                .collect();
            let count = held.iter().filter(|h| **h).count();
            let turns = (0..RING)
                .filter(|k| held[*k] != held[(k + 1) % RING])
                .count();
            !(count == 0 || count == RING || (count == RING / 2 && turns == 2))
        }
    }
}

/// Whether the eye sees `p`: nothing of the object nearer on its ray.
fn visible(body: &Body, p: Vector) -> bool {
    body.hit(p).is_none_or(|hit| hit.t >= 1.0 - SEEN)
}

/// Where `holds` holds over `[from, to]`, looked at in `steps` even steps and each end found by
/// halving.
fn runs(steps: usize, (from, to): (f64, f64), holds: impl Fn(f64) -> bool) -> Vec<[f64; 2]> {
    let at = |k: usize| from + (to - from) * (k as f64 + 0.5) / steps as f64;
    let states: Vec<bool> = (0..steps).map(|k| holds(at(k))).collect();
    let edge = (to - from) * 1e-7;
    let mut out = Vec::new();
    let mut start = None;
    if states[0] {
        start = Some(if holds(from + edge) {
            from
        } else {
            halved(from + edge, at(0), &holds)
        });
    }
    for k in 1..steps {
        match (states[k - 1], states[k]) {
            (false, true) => start = Some(halved(at(k - 1), at(k), &holds)),
            (true, false) => {
                let end = halved(at(k - 1), at(k), &holds);
                out.extend(start.take().map(|s| [s, end]));
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        let end = if holds(to - edge) {
            to
        } else {
            halved(at(steps - 1), to - edge, &holds)
        };
        out.push([s, end]);
    }
    out
}

/// Where `holds` changes between `a` and `b` (which it tells apart).
fn halved(mut a: f64, mut b: f64, holds: &impl Fn(f64) -> bool) -> f64 {
    let at_a = holds(a);
    for _ in 0..HALVINGS {
        let middle = (a + b) / 2.0;
        if holds(middle) == at_a {
            a = middle;
        } else {
            b = middle;
        }
    }
    (a + b) / 2.0
}

/// `segments` on one line joined where they overlap or meet.
pub(crate) fn merged(segments: Vec<[Point; 2]>) -> Vec<[Point; 2]> {
    let mut out: Vec<[Point; 2]> = Vec::new();
    let mut left: Vec<[Point; 2]> = (segments.into_iter())
        .filter(|[a, b]| distance(*a, *b) > ONE_LINE)
        .collect();
    while let Some(first) = left.pop() {
        let [a, b] = first;
        let long = distance(a, b);
        let u = [(b[0] - a[0]) / long, (b[1] - a[1]) / long];
        let along = |p: Point| (p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1];
        let off = |p: Point| ((p[0] - a[0]) * u[1] - (p[1] - a[1]) * u[0]).abs();
        let (mut same, rest): (Vec<[Point; 2]>, Vec<[Point; 2]>) = left
            .into_iter()
            .partition(|[p, q]| off(*p) <= ONE_LINE && off(*q) <= ONE_LINE);
        left = rest;
        same.push(first);
        let mut spans: Vec<[f64; 2]> = (same.iter())
            .map(|[p, q]| {
                let (s, t) = (along(*p), along(*q));
                [s.min(t), s.max(t)]
            })
            .collect();
        spans.sort_by(|x, y| x[0].total_cmp(&y[0]));
        let mut joined: Vec<[f64; 2]> = Vec::new();
        for span in spans {
            match joined.last_mut() {
                Some(last) if span[0] <= last[1] + ONE_LINE => last[1] = last[1].max(span[1]),
                _ => joined.push(span),
            }
        }
        let point = |s: f64| [a[0] + u[0] * s, a[1] + u[1] * s];
        out.extend(joined.into_iter().map(|[s, t]| [point(s), point(t)]));
    }
    out
}

/// `corners` each once.
fn distinct(corners: Vec<Local>) -> Vec<Local> {
    let mut out: Vec<Local> = Vec::new();
    for corner in corners {
        if !out.iter().any(|p| norm(sub(*p, corner)) <= 1e-7) {
            out.push(corner);
        }
    }
    out
}
