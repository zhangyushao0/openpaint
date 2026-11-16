//! A line in space along a parameter (S156k): a form's line drawn through, and the exact line where
//! two forms meet. A straight piece, a circle's arc, or round a cylinder where its side meets
//! another form: at each turn the line along its side carried to that form's surface, the length
//! solved from a quadratic (Drawabox's sections: each one's point, exact).

use crate::kinds::contours::sections::Circle;
use crate::maths::vector::{Vector, add, dot, sub, times};

/// A cylinder's side another's lines are carried to: a point of its axis, its axis (one long),
/// its radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Side {
    pub(crate) at: Vector,
    pub(crate) axis: Vector,
    pub(crate) radius: f64,
}

impl Side {
    /// How far along the line from `from` along `way` (one long) it first meets it; none where it
    /// misses: `|w + s d|² = r²` across the axis, its nearer root.
    fn reached(&self, from: Vector, way: Vector) -> Option<f64> {
        let across = |v: Vector| sub(v, times(self.axis, dot(v, self.axis)));
        let (w, d) = (across(sub(from, self.at)), across(way));
        let r = self.radius;
        let [a, b, c] = [dot(d, d), dot(w, d), dot(w, w) - r * r];
        let disc = b * b - a * c;
        (a > 0.0 && disc >= 0.0).then(|| (-b - disc.sqrt()) / a)
    }
}

/// A line in space along its parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Track {
    /// From one end to the other, 0 to 1.
    Line([Vector; 2]),
    /// A circle's arc from one turn to the other (radians), 0 to their difference.
    Arc(Circle, [f64; 2]),
    /// Round a cylinder's rim `rim` (at each turn, 0 to a full turn) the line along its side
    /// `way` (one long) carried to where it meets `into`.
    Into {
        rim: Circle,
        way: Vector,
        into: Side,
    },
    /// Where a ball sunk into a cylinder's side meets it: round its rim `rim` (its centre on the
    /// axis, `u` towards the ball) at `middle ± spread cos s` (`s` 0 to a full turn), the line
    /// along its side `way` (its axis, one long) to the ball, nearer the rim where `sin s < 0`.
    Lens {
        rim: Circle,
        way: Vector,
        ball: (Vector, f64),
        middle: f64,
        spread: f64,
    },
}

impl Track {
    /// A whole circle.
    pub(crate) const fn whole(circle: Circle) -> Self {
        Self::Arc(circle, [0.0, std::f64::consts::TAU])
    }

    /// Its parameter's span, and whether it closes on itself.
    pub(crate) fn span(&self) -> (f64, bool) {
        let tau = std::f64::consts::TAU;
        match self {
            Self::Line(_) => (1.0, false),
            Self::Arc(_, [from, to]) => ((to - from).abs(), (to - from).abs() >= tau - 1e-12),
            Self::Into { .. } | Self::Lens { .. } => (tau, true),
        }
    }

    /// Its point at `t`.
    pub(crate) fn at(&self, t: f64) -> Vector {
        match *self {
            Self::Line([a, b]) => add(a, times(sub(b, a), t)),
            Self::Arc(circle, [from, to]) => circle.at(from + t * (to - from).signum()),
            Self::Into { rim, way, into } => {
                let from = rim.at(t);
                add(from, times(way, into.reached(from, way).unwrap_or(0.0)))
            }
            Self::Lens {
                rim,
                way,
                ball: (centre, radius),
                middle,
                spread,
            } => {
                let from = rim.at(middle + spread * t.cos());
                let w = sub(from, centre);
                let along = dot(w, way);
                let room = (along * along - dot(w, w) + radius * radius).max(0.0);
                let side = if t.sin() < 0.0 { 1.0 } else { -1.0 };
                add(from, times(way, -along - side * room.sqrt()))
            }
        }
    }
}
