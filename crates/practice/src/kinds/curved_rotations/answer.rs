//! A curved object's exact answer (S154f, `body.rs`): its lines as the eye sees them, hidden by its
//! own parts, never found on a picture. Every line that may show is known: each frustum's two
//! outlines along its side (where planes through the eye touch it, as S93's and unit 4's), each
//! dome's outline (where the eye's lines of sight touch its ball, on its outer side), the rim of
//! each section where the surface turns (a joint's, an end's flat face's), a spout's outlines and
//! tip, a handle's edges, and where a part on the side meets the body (each found by a ray along
//! the part's side into the body: exact at every point). Along each, where it lies on the surface
//! (inside no other part) and where the eye sees it are found at even steps, each change to the
//! last bit by halving; then projected: seen whole, hidden dashed.

use crate::kinds::curved_rotations::body::{Dome, Frustum, Parts};
use crate::kinds::solids::solid_answer::{Stretch, View};
use crate::maths::path::{Point, distance, length};
use crate::maths::vector::{Vector, across, add, dot, norm, sub, times};

/// Steps along a line (frame units: a pixel and a half of a 400-pixel frame), and at least.
const STEP: f64 = 0.004;
const LEAST_STEPS: usize = 24;
/// Halvings that find where a stretch ends.
const HALVINGS: usize = 48;
/// A point is seen when nothing lies nearer on its ray than this share of the way.
const SEEN: f64 = 1e-6;
/// A stretch shorter than this (frame units, six pixels of a 400-pixel frame) is a crumb no pen
/// draws: left out, seen or hidden.
const CRUMB: f64 = 0.015;

/// What a line of the answer belongs to: a segment (its outlines), a section (its rim), or what
/// stands on the side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Segment(usize),
    Section(usize),
    Attachment,
}

/// A curved line of the answer in stretches, seen or hidden, and what it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvedLine {
    pub stretches: Vec<Stretch>,
    pub owner: Owner,
}

/// A curved object as the eye sees it, in frame units: its straight lines seen and where its own
/// parts hide them, its curves in stretches, and how long each owner's lines show.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CurvedAnswer {
    pub sides: Vec<[Point; 2]>,
    pub hidden: Vec<[Point; 2]>,
    pub curves: Vec<CurvedLine>,
    pub seen: Vec<(Owner, f64)>,
}

impl CurvedAnswer {
    /// How long `owner`'s lines show, frame units.
    #[must_use]
    pub fn shown(&self, owner: Owner) -> f64 {
        (self.seen.iter())
            .filter(|(o, _)| *o == owner)
            .map(|(_, long)| long)
            .sum()
    }

    /// Every point of it, seen or hidden.
    pub(crate) fn points(&self) -> impl Iterator<Item = Point> + '_ {
        let straight =
            (self.sides.iter().chain(&self.hidden)).flat_map(|side| side.iter().copied());
        let curves = (self.curves.iter())
            .flat_map(|c| c.stretches.iter().flat_map(|s| s.points.iter().copied()));
        straight.chain(curves)
    }
}

/// A line of the object that may show.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Line {
    /// A straight edge or outline: its ends.
    Straight([Vector; 2]),
    /// A circle round `centre` through `centre + u` and `centre + v`; where it is a dome's outline,
    /// only on its outer side.
    Circle {
        centre: Vector,
        u: Vector,
        v: Vector,
        dome: Option<Dome>,
    },
    /// Where a spout meets the body: each line along its side from its tip into the body.
    Spout(Frustum),
    /// Where a bar meets the body: each line along it from its outer end's cross-section (its
    /// corners in order round) by `inward` into the body.
    Bar {
        corners: [Vector; 4],
        inward: Vector,
    },
}

impl Line {
    /// Its parameter's span, and whether it closes on itself.
    const fn span(&self) -> (f64, bool) {
        match self {
            Self::Straight(_) => (1.0, false),
            Self::Circle { .. } | Self::Spout(_) => (std::f64::consts::TAU, true),
            Self::Bar { .. } => (4.0, true),
        }
    }

    /// Its point at `t`; none where a ray into the body finds none.
    fn at(&self, parts: &Parts, t: f64) -> Option<Vector> {
        match *self {
            Self::Straight([a, b]) => Some(add(a, times(sub(b, a), t))),
            Self::Circle { centre, u, v, .. } => {
                Some(add(centre, add(times(u, t.cos()), times(v, t.sin()))))
            }
            Self::Spout(spout) => {
                let [first, second] = across(spout.axis);
                let round = add(times(first, t.cos()), times(second, t.sin()));
                let top = add(spout.centre, times(spout.axis, spout.length));
                let tip = add(top, times(round, spout.radii[1]));
                let root = add(spout.centre, times(round, spout.radii[0]));
                into(parts, tip, sub(root, tip))
            }
            Self::Bar { corners, inward } => {
                let side = (t.floor() as usize).min(3);
                let [a, b] = [corners[side], corners[(side + 1) % 4]];
                into(parts, add(a, times(sub(b, a), t - side as f64)), inward)
            }
        }
    }
}

/// Where the line from `from` along `d` first meets the main body, within its length.
fn into(parts: &Parts, from: Vector, d: Vector) -> Option<Vector> {
    let hit = parts.cast((from, d), true)?;
    (hit.t <= 1.0).then(|| add(from, times(d, hit.t)))
}

/// How a point of a line stands: on no surface (inside a part, or off a dome), hidden, seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Off,
    Hidden,
    Seen,
}

fn state(parts: &Parts, line: &Line, t: f64) -> State {
    let Some(p) = line.at(parts, t) else {
        return State::Off;
    };
    let off_dome = matches!(line, Line::Circle { dome: Some(dome), .. } if !dome.outer(p));
    if off_dome || parts.inside(p) {
        State::Off
    } else if parts.hit(p).is_none_or(|hit| hit.t >= 1.0 - SEEN) {
        State::Seen
    } else {
        State::Hidden
    }
}

/// `lines` of `parts` as the eye sees them through `view`, each with what it belongs to.
pub(crate) fn seen(parts: &Parts, view: View, lines: &[(Line, Owner)]) -> CurvedAnswer {
    let mut answer = CurvedAnswer::default();
    for (line, owner) in lines {
        let (span, closed) = line.span();
        let point = |t: f64| line.at(parts, t).map(|p| view.at(p));
        let long = (0..16)
            .filter_map(|k| {
                let [a, b] = [k, k + 1].map(|i| point(span * f64::from(i) / 16.0));
                a.zip(b).map(|(a, b)| distance(a, b))
            })
            .sum::<f64>();
        let steps = ((long / STEP) as usize).max(LEAST_STEPS);
        let runs = runs(steps, (span, closed), |t| state(parts, line, t));
        let mut stretches = Vec::new();
        for (from, to, state) in runs {
            if state == State::Off {
                continue;
            }
            let hidden = state == State::Hidden;
            if let Line::Straight(_) = line {
                let ends = [from, to].map(|t| point(t).unwrap_or([0.0; 2]));
                let long = distance(ends[0], ends[1]);
                if long < CRUMB {
                    continue;
                } else if hidden {
                    answer.hidden.push(ends);
                } else {
                    answer.sides.push(ends);
                    answer.seen.push((*owner, long));
                }
                continue;
            }
            let pieces = ((to - from) / span * 160.0).ceil().max(2.0) as usize;
            let points: Vec<Point> = (0..=pieces)
                .filter_map(|k| point((from + (to - from) * k as f64 / pieces as f64) % span))
                .collect();
            let long = length(&points);
            if long < CRUMB {
                continue;
            }
            if !hidden {
                answer.seen.push((*owner, long));
            }
            stretches.push(Stretch { points, hidden });
        }
        if !matches!(line, Line::Straight(_)) {
            answer.curves.push(CurvedLine {
                stretches,
                owner: *owner,
            });
        }
    }
    answer
}

/// The runs of `state` over `[0, span]`, looked at in `steps` even steps and each change found by
/// halving; a closed line's last run joins its first where alike (its end past `span`).
fn runs<S: Copy + PartialEq>(
    steps: usize,
    (span, closed): (f64, bool),
    state: impl Fn(f64) -> S,
) -> Vec<(f64, f64, S)> {
    let at = |k: usize| span * (k as f64 + 0.5) / steps as f64;
    let states: Vec<S> = (0..steps).map(|k| state(at(k))).collect();
    let edge = span * 1e-9;
    let mut starts: Vec<(f64, S)> = Vec::new();
    // The first run from the line's start, or from where its state changes before the first step.
    let first = if closed || state(edge) == states[0] {
        0.0
    } else {
        starts.push((0.0, state(edge)));
        halved(edge, at(0), &state)
    };
    starts.push((first, states[0]));
    for k in 1..steps {
        if states[k] != states[k - 1] {
            starts.push((halved(at(k - 1), at(k), &state), states[k]));
        }
    }
    let last = states[steps - 1];
    if !closed && state(span - edge) != last {
        starts.push((
            halved(at(steps - 1), span - edge, &state),
            state(span - edge),
        ));
    }
    let mut out: Vec<(f64, f64, S)> = (starts.windows(2))
        .map(|pair| (pair[0].0, pair[1].0, pair[0].1))
        .collect();
    if let Some((from, held)) = starts.last().copied() {
        out.push((from, span, held));
    }
    if closed && out.len() > 1 && out[0].2 == out[out.len() - 1].2 {
        let (_, to, _) = out.remove(0);
        if let Some(last) = out.last_mut() {
            last.1 = span + to;
        }
    }
    out.retain(|(from, to, _)| to > from);
    out
}

/// Where `state` changes between `a` and `b` (which it tells apart).
fn halved<S: Copy + PartialEq>(mut a: f64, mut b: f64, state: &impl Fn(f64) -> S) -> f64 {
    let at_a = state(a);
    for _ in 0..HALVINGS {
        let middle = (a + b) / 2.0;
        if state(middle) == at_a {
            a = middle;
        } else {
            b = middle;
        }
    }
    (a + b) / 2.0
}

/// A frustum's two outlines along its side, where planes through the eye touch it (none where the
/// eye looks along it): with `n` its rim's outward way, where `h n·c + h r0 + (r0 - r1) a·c = 0`.
pub(crate) fn outlines(frustum: &Frustum) -> Vec<[Vector; 2]> {
    let c = frustum.centre;
    let [u, v] = across(frustum.axis);
    let [r0, r1] = frustum.radii;
    let h = frustum.length;
    let wanted = -(h * r0 + (r0 - r1) * dot(frustum.axis, c)) / h;
    let (a, b) = (dot(u, c), dot(v, c));
    let reach = a.hypot(b).max(f64::MIN_POSITIVE);
    if (wanted / reach).abs() > 1.0 {
        return Vec::new();
    }
    let (phi, spread) = (b.atan2(a), (wanted / reach).acos());
    let top = add(c, times(frustum.axis, h));
    [phi - spread, phi + spread]
        .map(|t| {
            let out = add(times(u, t.cos()), times(v, t.sin()));
            [add(c, times(out, r0)), add(top, times(out, r1))]
        })
        .to_vec()
}

/// A dome's outline: the circle where the eye's lines of sight touch its ball (its centre, two
/// radii square to each other), on its outer side only.
pub(crate) fn dome_outline(dome: &Dome) -> Line {
    let c = dome.ball.centre;
    let far = norm(c).max(f64::MIN_POSITIVE);
    let share = (dome.ball.radius / far).powi(2);
    let centre = times(c, 1.0 - share);
    let radius = dome.ball.radius * (1.0 - share).max(0.0).sqrt();
    let [u, v] = across(times(c, 1.0 / far));
    Line::Circle {
        centre,
        u: times(u, radius),
        v: times(v, radius),
        dome: Some(*dome),
    }
}
