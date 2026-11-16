//! A placement's plan as printed in the original's frame (S153, the accepted mock; V大's three
//! views, the plan from above and the front view under it): the ground from above, each footprint
//! shaded and outlined (a cylinder's circle), the near corner's dot; under it the front view, a
//! baseline as wide as the ground and each piece's height as a shaded rectangle straight under its
//! footprint (a turned box's nearest corner its upright), and the unit beside them, a bar with
//! its ends ticked; at 稳 the unit along the ground's near edge too, between two dots. One scale
//! for both views.

use crate::kinds::placements::plan::{Footprint, Plan};
use crate::maths::path::Point;

/// The margin round the views and the gap between them, frame units; the most the ground's width
/// takes; the unit bar's place left of the ground and its ticks' half length.
const MARGIN: f64 = 0.08;
const GAP: f64 = 0.08;
const WIDEST: f64 = 0.72;
const BAR: f64 = 0.05;
const TICK: f64 = 0.012;
/// A circle drawn in this many pieces.
const PIECES: usize = 48;

/// What the plan prints: a line, a shaded area, a dot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Printed {
    Line(Vec<Point>),
    Area(Vec<Point>),
    Dot(Point),
}

/// Where the plan lies in its frame: frame units a unit, the ground's far left corner, the front
/// view's baseline.
#[derive(Debug, Clone, Copy)]
struct Laid {
    scale: f64,
    corner: Point,
    baseline: f64,
}

impl Laid {
    fn of(plan: &Plan) -> Self {
        let [width, depth] = plan.ground;
        let tallest = (plan.pieces.iter()).fold(0.0, |most: f64, piece| most.max(piece.height));
        let room = 1.0 - 2.0 * MARGIN - GAP;
        let scale = (WIDEST / width).min(room / (depth + tallest));
        let tall = (depth + tallest) * scale + GAP;
        let corner = [(1.0 - width * scale) / 2.0 + BAR / 2.0, (1.0 - tall) / 2.0];
        Self {
            scale,
            corner,
            baseline: corner[1] + tall,
        }
    }

    /// The plan's point `[along, away]` in the frame.
    fn at(&self, [along, away]: Point, depth: f64) -> Point {
        [
            self.corner[0] + along * self.scale,
            self.corner[1] + (depth - away) * self.scale,
        ]
    }
}

/// What the plan of `plan` prints, the unit along the near edge where `along`.
pub(crate) fn printed(plan: &Plan, along: bool) -> Vec<Printed> {
    let laid = Laid::of(plan);
    let [width, depth] = plan.ground;
    let at = |p: Point| laid.at(p, depth);
    let ground = [[0.0, 0.0], [width, 0.0], [width, depth], [0.0, depth]].map(at);
    let mut out = vec![Printed::Line(closed(&ground))];
    for piece in &plan.pieces {
        let outline: Vec<Point> = match piece.footprint {
            Footprint::Round { cell, side } => {
                let [w, d] = plan.cell();
                let middle = [(cell[0] as f64 + 0.5) * w, (cell[1] as f64 + 0.5) * d];
                circle(at(middle), side / 2.0 * laid.scale)
            }
            Footprint::Cells { .. } | Footprint::Turned { .. } => {
                plan.corners(piece).map(at).to_vec()
            }
        };
        out.push(Printed::Area(outline.clone()));
        out.push(Printed::Line(closed(&outline)));
    }
    out.extend(front(plan, &laid));
    out.push(Printed::Dot(at([0.0, 0.0])));
    if along {
        out.push(Printed::Dot(at([1.0, 0.0])));
    }
    out
}

/// The front view: the baseline, each piece's rectangle (a turned box's nearest corner's upright
/// in it), the unit bar with its ends ticked.
fn front(plan: &Plan, laid: &Laid) -> Vec<Printed> {
    let base = laid.baseline;
    let x = |along: f64| laid.corner[0] + along * laid.scale;
    let mut out = vec![Printed::Line(vec![
        [x(0.0), base],
        [x(plan.ground[0]), base],
    ])];
    for piece in &plan.pieces {
        let [from, to] = plan.span(piece).map(x);
        let top = base - piece.height * laid.scale;
        let rectangle = [[from, base], [to, base], [to, top], [from, top]];
        out.push(Printed::Area(rectangle.to_vec()));
        out.push(Printed::Line(closed(&rectangle)));
        if let Footprint::Turned { .. } = piece.footprint {
            let corners = plan.corners(piece);
            let nearest = (corners.iter()).min_by(|a, b| a[1].total_cmp(&b[1]));
            if let Some(nearest) = nearest {
                out.push(Printed::Line(vec![
                    [x(nearest[0]), base],
                    [x(nearest[0]), top],
                ]));
            }
        }
    }
    let bar = laid.corner[0] - BAR;
    let unit_top = base - laid.scale;
    out.push(Printed::Line(vec![[bar, base], [bar, unit_top]]));
    for y in [base, unit_top] {
        out.push(Printed::Line(vec![[bar - TICK, y], [bar + TICK, y]]));
    }
    out
}

/// `points` with the first again at the end.
fn closed(points: &[Point]) -> Vec<Point> {
    points.iter().chain(points.first()).copied().collect()
}

/// The circle about `middle` of `radius`, in `PIECES` pieces (the first point not repeated).
fn circle(middle: Point, radius: f64) -> Vec<Point> {
    (0..PIECES)
        .map(|k| {
            let turn = k as f64 / PIECES as f64 * std::f64::consts::TAU;
            [
                middle[0] + radius * turn.cos(),
                middle[1] + radius * turn.sin(),
            ]
        })
        .collect()
}
