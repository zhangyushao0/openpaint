//! A placement's exact answer and V大's construction (S153).
//!
//! The answer: the pieces in the eye's view, each box a convex hull and each cylinder a round
//! part of one body, as the eye sees them (`object_answer::seen`, unit 4's): every edge and outline
//! where nothing nearer hides it, and where another piece or its own faces hide it.
//!
//! The construction, found on the copy's frame from what it gives alone, as the learner finds it:
//! the ground's diagonals and the lines through its cells' corners (its halves and quarters, as
//! the diagonals halve it; the plan laid on the ground as V大's Ctrl+T lays it, the square-to-
//! quadrilateral map); each footprint's corners there; each piece's height carried from the
//! upright at the near corner: the line from the upright's foot through the piece's nearest corner
//! to the horizon, back from the height on the upright to that point, crossing the piece's
//! upright at its top; the top closed by its edges toward their vanishing points. At 稳, where the
//! upright is not given, its height on the near corner is the eye's (its exact place shown).

use crate::geometry::ellipse::{apply, homography};
use crate::kinds::objects::object_body::{Body, Hull, Placed, Plane, Round};
use crate::kinds::placements::plan::{Footprint, Plan};
use crate::kinds::placements::sight::Sight;
use crate::kinds::solids::solid_answer::Square;
use crate::maths::path::{Point, distance, meet};
use crate::maths::vector::{Vector, add, cross, dot, sub, times, unit};

/// The body of `plan`'s pieces as `sight` views them (the eye's view: right, up, forward).
pub(crate) fn body(plan: &Plan, sight: &Sight) -> Body {
    let up = sight.viewed([0.0, 1.0, 0.0]);
    let (mut hulls, mut rounds) = (Vec::new(), Vec::new());
    for piece in &plan.pieces {
        let feet = (plan.corners(piece)).map(|p| sight.viewed(sight.world(p, 0.0)));
        match piece.footprint {
            Footprint::Round { .. } => rounds.push(Round {
                base: Square {
                    corner: feet[0],
                    u: sub(feet[1], feet[0]),
                    v: sub(feet[3], feet[0]),
                },
                axis: up,
                length: piece.height,
                cone: false,
            }),
            Footprint::Cells { .. } | Footprint::Turned { .. } => {
                hulls.push(hull(feet, times(up, piece.height), hulls.len()));
            }
        }
    }
    Body {
        placed: Placed {
            origin: [0.0; 3],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        hulls,
        rounds,
    }
}

/// A box standing on `feet` (in order round), `rise` up to its top, the `index`th of its body.
fn hull(feet: [Vector; 4], rise: Vector, index: usize) -> Hull {
    let tops = feet.map(|p| add(p, rise));
    let middle = (feet.iter().chain(&tops)).fold([0.0; 3], |sum, p| add(sum, times(*p, 0.125)));
    let plane = |normal: Vector, at: Vector| {
        let normal = unit(normal);
        let outward = if dot(normal, sub(middle, at)) > 0.0 {
            times(normal, -1.0)
        } else {
            normal
        };
        Plane {
            normal: outward,
            offset: dot(outward, at),
        }
    };
    let mut planes = vec![plane(rise, tops[0]), plane(times(rise, -1.0), feet[0])];
    for k in 0..4 {
        planes.push(plane(cross(sub(feet[(k + 1) % 4], feet[k]), rise), feet[k]));
    }
    let mut edges = Vec::new();
    for k in 0..4 {
        let next = (k + 1) % 4;
        edges.extend([
            [feet[k], feet[next]],
            [tops[k], tops[next]],
            [feet[k], tops[k]],
        ]);
    }
    let surfaces = (0..planes.len()).map(|k| (index * 6 + k) as u32).collect();
    Hull {
        planes,
        corners: feet.iter().chain(&tops).copied().collect(),
        edges,
        surfaces,
    }
}

/// What the copy's frame gives that the construction starts from: the ground's corners (near,
/// right, far, left: the plan's corners from its near one round), the horizon, the unit upright
/// at the near corner (foot and top; at 稳 its exact place, which the learner judges), and the
/// third vanishing point where the uprights meet (稳).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Given {
    pub(crate) ground: [Point; 4],
    pub(crate) horizon: f64,
    pub(crate) upright: [Point; 2],
    pub(crate) third: Option<Point>,
}

/// A piece's height carried from the upright: its nearest corner on the ground (its corner's
/// place among its footprint's), its top over it, and the lines that find it.
#[derive(Debug, Clone, PartialEq)]
pub struct Carried {
    pub corner: usize,
    pub foot: Point,
    pub top: Point,
    pub lines: Vec<[Point; 2]>,
}

/// V大's construction on the copy's frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Construction {
    /// The ground's two diagonals, then the lines through its cells' corners.
    pub grid: Vec<[Point; 2]>,
    /// The upright at the near corner as tall as the tallest piece.
    pub upright: [Point; 2],
    /// Each piece's footprint on the ground, its corners in the plan's order.
    pub footprints: Vec<[Point; 4]>,
    /// Each piece's height carried.
    pub carried: Vec<Carried>,
    /// Each piece's top, closed from its carried corner, its corners in the plan's order.
    pub tops: Vec<[Point; 4]>,
    /// A turned piece's edges from its carried corner run on to their own vanishing points.
    pub runs: Vec<[Point; 2]>,
}

/// The construction of `plan` on `given`; none where a line it needs runs parallel to another.
pub(crate) fn construction(plan: &Plan, given: &Given) -> Option<Construction> {
    let [width, depth] = plan.ground;
    let to = homography(given.ground);
    let on = |[u, v]: Point| apply(&to, [u / width, v / depth]);
    let g = given.ground;
    let mut grid = vec![[g[0], g[2]], [g[1], g[3]]];
    let cells = plan.cells;
    for k in 1..cells {
        let share = k as f64 / cells as f64;
        grid.push([on([share * width, 0.0]), on([share * width, depth])]);
        grid.push([on([0.0, share * depth]), on([width, share * depth])]);
    }
    let tallest = (plan.pieces.iter()).fold(0.0, |most: f64, piece| most.max(piece.height));
    let mut built = Construction {
        grid,
        upright: [given.upright[0], raised(given, tallest)],
        footprints: Vec::new(),
        carried: Vec::new(),
        tops: Vec::new(),
        runs: Vec::new(),
    };
    for piece in &plan.pieces {
        let feet = plan.corners(piece).map(on);
        // The nearest corner: the lowest on the frame, the ground lying below the eye.
        let corner = (0..4)
            .max_by(|a, b| feet[*a][1].total_cmp(&feet[*b][1]))
            .unwrap_or(0);
        let carried = carry(given, (corner, feet[corner]), piece.height)?;
        let tops = closed(given, &feet, (corner, carried.top))?;
        if matches!(piece.footprint, Footprint::Turned { .. }) {
            for next in [(corner + 1) % 4, (corner + 3) % 4] {
                let run = vanishing(given, [feet[corner], feet[next]]);
                built.runs.extend(run.map(|v| [feet[corner], v]));
            }
        }
        built.footprints.push(feet);
        built.carried.push(carried);
        built.tops.push(tops);
    }
    Some(built)
}

/// The point `height` up the upright at the near corner: along it evenly at two points, by the
/// cross-ratio its foot, its unit top and the third vanishing point keep at three.
pub(crate) fn raised(given: &Given, height: f64) -> Point {
    let [foot, top] = given.upright;
    let Some(third) = given.third else {
        return [
            foot[0] + (top[0] - foot[0]) * height,
            foot[1] + (top[1] - foot[1]) * height,
        ];
    };
    let long = distance(foot, top).max(f64::MIN_POSITIVE);
    let up = [(top[0] - foot[0]) / long, (top[1] - foot[1]) / long];
    let to_third = (third[0] - foot[0]) * up[0] + (third[1] - foot[1]) * up[1];
    let k = to_third / long - 1.0;
    let along = to_third * height / (height + k);
    [foot[0] + up[0] * along, foot[1] + up[1] * along]
}

/// The upright through `p`: straight up at two points, toward the third vanishing point at three.
fn upright_at(given: &Given, p: Point) -> [Point; 2] {
    given
        .third
        .map_or([p, [p[0], p[1] - 1.0]], |third| [p, third])
}

/// Where the line through `edge` meets the horizon; none where it runs along it.
fn vanishing(given: &Given, edge: [Point; 2]) -> Option<Point> {
    meet(edge, [[0.0, given.horizon], [1.0, given.horizon]])
}

/// The point over `p` on the line from `from` heading as `edge` does (toward its vanishing point,
/// or along it where it vanishes nowhere).
fn lifted(given: &Given, from: Point, edge: [Point; 2], p: Point) -> Option<Point> {
    let toward = vanishing(given, edge).unwrap_or([
        from[0] + edge[1][0] - edge[0][0],
        from[1] + edge[1][1] - edge[0][1],
    ]);
    meet([from, toward], upright_at(given, p))
}

/// The height `height` carried from the near corner's upright to `foot`, its footprint's corner
/// `corner`.
fn carry(given: &Given, (corner, foot): (usize, Point), height: f64) -> Option<Carried> {
    let near = given.upright[0];
    let up = raised(given, height);
    if distance(near, foot) <= 1e-9 {
        return Some(Carried {
            corner,
            foot,
            top: up,
            lines: Vec::new(),
        });
    }
    let top = lifted(given, up, [near, foot], foot)?;
    let ends = vanishing(given, [near, foot]).map_or([foot, top], |v| [v, v]);
    Some(Carried {
        corner,
        foot,
        top,
        lines: vec![[near, ends[0]], [up, ends[1]]],
    })
}

/// The top over `feet` closed from its corner `corner`'s `top`: each corner beside it lifted along
/// its edge from it, the far one along the edge from a corner beside it.
fn closed(given: &Given, feet: &[Point; 4], (corner, top): (usize, Point)) -> Option<[Point; 4]> {
    let [next, far, last] = [1, 2, 3].map(|k| (corner + k) % 4);
    let mut tops = [top; 4];
    tops[next] = lifted(given, top, [feet[corner], feet[next]], feet[next])?;
    tops[last] = lifted(given, top, [feet[corner], feet[last]], feet[last])?;
    tops[far] = lifted(given, tops[next], [feet[next], feet[far]], feet[far])?;
    Some(tops)
}
