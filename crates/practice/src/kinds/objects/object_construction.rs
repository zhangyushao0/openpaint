//! How V大's 点定位 finds an object's corners on the picture (S94), from the box as given and
//! completed, by lines alone: never a point guessed. A corner `(a, b, c)` of the box's own
//! coordinates (each a half, a third or a quarter, `object_design.rs`):
//!
//! - its width `a` as an upright on the front face along `a`, its depth `c` as one on the front
//!   face along `c`: a half where the face's diagonals cross, a quarter where the half's do, a
//!   third where the near upright's thirds run to the vanishing point cross a diagonal (S90's);
//! - its height `b` measured on the near upright (an upright at one depth shows its thirds and
//!   quarters as they are) and run to each face's vanishing point, meeting the uprights there;
//! - the two front-face points run into depth, each to the other's vanishing point: they cross at
//!   the corner, on the box's surface or inside it, never in empty space.
//!
//! A round part's square end found so, then S93's diagonals and middles; a cone's point where its
//! opposite face's diagonals cross. Each change's corners make a stage, the biggest change's
//! first. What it finds is the object in space projected (`tests/objects/`).

use crate::geometry::ellipse::square_on;
use crate::geometry::perspective::{Direction, PerspectiveBox};
use crate::kinds::objects::object_answer::merged;
use crate::kinds::objects::object_design::{Local, SAME};
use crate::kinds::solids::solid::{diagonals, middle_lines};
use crate::maths::path::{Point, distance, meet};

/// Half a tick across the near upright where a height is measured (frame units).
const TICK: f64 = 0.015;

/// One change's construction: its lines, the corners they find (and where those lie in the box's
/// own coordinates), a round part's middles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stage {
    pub lines: Vec<[Point; 2]>,
    pub found: Vec<Point>,
    pub at: Vec<[f64; 3]>,
    pub middles: Vec<Point>,
}

/// What a change asks to be found: its corners, a round part's square ends (corners in order
/// round), a cone's opposite face (its diagonals find the point).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Wanted {
    pub(crate) corners: Vec<Local>,
    pub(crate) ends: Vec<[Local; 4]>,
    pub(crate) points: Vec<[Local; 4]>,
}

/// The construction of `wanted`, stage by stage, on the box `seen`.
pub(crate) fn construction(seen: &PerspectiveBox, wanted: &[Wanted]) -> Vec<Stage> {
    let mut builder = Builder {
        seen,
        lines: Vec::new(),
    };
    let mut stages = Vec::new();
    for want in wanted {
        let before = builder.lines.len();
        let mut stage = Stage::default();
        for corner in &want.corners {
            stage.found.push(builder.point(*corner));
            stage.at.push(*corner);
        }
        for end in &want.ends {
            let corners = end.map(|p| builder.point(p));
            builder.outline(&corners);
            let guide = square_on(corners);
            builder.lines.extend(diagonals(&corners));
            builder.lines.extend(middle_lines(&guide));
            stage.middles.extend(guide.touches);
        }
        for face in &want.points {
            let corners = face.map(|p| builder.point(p));
            builder.outline(&corners);
            let [first, second] = diagonals(&corners);
            builder.lines.extend([first, second]);
            if let Some(point) = meet(first, second) {
                let centre: [f64; 3] = std::array::from_fn(|k| (face[0][k] + face[2][k]) / 2.0);
                stage.found.push(point);
                stage.at.push(centre);
            }
        }
        stage.lines = builder.lines[before..].to_vec();
        stages.push(stage);
    }
    for stage in &mut stages {
        stage.lines = drawn(seen, std::mem::take(&mut stage.lines));
    }
    stages
}

/// `lines` as drawn: each once, joined where they run on, none along the box's own edges.
fn drawn(seen: &PerspectiveBox, lines: Vec<[Point; 2]>) -> Vec<[Point; 2]> {
    let on_box = |[a, b]: &[Point; 2]| {
        seen.edges().iter().any(|(p, q, _)| {
            let off = |x: Point| {
                let long = distance(*p, *q).max(f64::MIN_POSITIVE);
                ((x[0] - p[0]) * (q[1] - p[1]) - (x[1] - p[1]) * (q[0] - p[0])).abs() / long
            };
            off(*a) <= 1e-9 && off(*b) <= 1e-9
        })
    };
    let mut kept = merged(lines);
    kept.retain(|line| !on_box(line) && distance(line[0], line[1]) > 1e-6);
    kept
}

struct Builder<'a> {
    seen: &'a PerspectiveBox,
    lines: Vec<[Point; 2]>,
}

impl Builder<'_> {
    /// The front face along `a` (`along` 0, towards the right vanishing point) or along `c` (2,
    /// towards the left one): its near bottom, far bottom, far top and near top corners.
    fn face(&self, along: usize) -> [Point; 4] {
        let c = self.seen.corners;
        if along == 0 {
            [c[0], c[1], c[3], c[2]]
        } else {
            [c[0], c[4], c[6], c[2]]
        }
    }

    /// The vanishing point the lines along `along` run to.
    fn vanishing(&self, along: usize) -> Point {
        let direction = if along == 0 {
            Direction::Right
        } else {
            Direction::Left
        };
        self.seen.vanishing[direction.index()].unwrap_or([0.5, self.seen.horizon])
    }

    /// The upright at `share` across the front face along `along`: its bottom, its top and the
    /// point the construction found it through (none for the face's own edges).
    fn upright(&mut self, along: usize, share: f64) -> ([Point; 2], Option<Point>) {
        let [b0, b1, t1, t0] = self.face(along);
        let near = |x: f64, y: f64| (x - y).abs() <= SAME;
        if near(share, 0.0) {
            return ([b0, t0], None);
        }
        if near(share, 1.0) {
            return ([b1, t1], None);
        }
        if near(share, 0.5) {
            return self.crossed([b0, b1, t1, t0]);
        }
        if near(share, 0.25) || near(share, 0.75) {
            // The half's upright drawn whole: it bounds the half whose diagonals find the quarter.
            let ([hb, ht], _) = self.upright(along, 0.5);
            self.lines.push([hb, ht]);
            let half = if share < 0.5 {
                [b0, hb, ht, t0]
            } else {
                [hb, b1, t1, ht]
            };
            return self.crossed(half);
        }
        // A third: the near upright's thirds run to the vanishing point cross the diagonal from
        // the near bottom to the far top at the face's thirds in depth.
        let third = [
            b0[0] + (t0[0] - b0[0]) * share,
            b0[1] + (t0[1] - b0[1]) * share,
        ];
        let point = self.vanishing(along);
        let crossing = meet([third, point], [b0, t1]).unwrap_or(third);
        self.lines.push([b0, t1]);
        self.lines.push([third, crossing]);
        (through(crossing, [b0, b1, t1, t0]), Some(crossing))
    }

    /// The upright through where `quad`'s diagonals cross, the diagonals drawn.
    fn crossed(&mut self, quad: [Point; 4]) -> ([Point; 2], Option<Point>) {
        let [first, second] = diagonals(&quad);
        self.lines.extend([first, second]);
        let centre = meet(first, second).unwrap_or(quad[0]);
        (through(centre, quad), Some(centre))
    }

    /// The point at `height` on the upright at `share` of the front face along `along`: the
    /// height measured on the near upright, run to the face's vanishing point; the upright drawn
    /// from the point that found it to the point found.
    fn on_face(&mut self, along: usize, share: f64, height: f64) -> Point {
        let [b0, _, _, t0] = self.face(along);
        let measured = [
            b0[0] + (t0[0] - b0[0]) * height,
            b0[1] + (t0[1] - b0[1]) * height,
        ];
        if height > SAME && height < 1.0 - SAME {
            // The measured share ticked across the near upright.
            self.lines.push([
                [measured[0] - TICK, measured[1]],
                [measured[0] + TICK, measured[1]],
            ]);
        }
        if share.abs() <= SAME {
            return measured;
        }
        let ([bottom, top], anchor) = self.upright(along, share);
        let found = if height.abs() <= SAME {
            bottom
        } else if (height - 1.0).abs() <= SAME {
            top
        } else {
            let point = self.vanishing(along);
            let found = meet([measured, point], [bottom, top]).unwrap_or(measured);
            self.lines.push([measured, found]);
            found
        };
        if let Some(anchor) = anchor {
            self.lines.push([anchor, found]);
        }
        found
    }

    /// The corner `[a, b, c]`: on a front face, or where the two front faces' points at its
    /// height run into depth cross.
    fn point(&mut self, [a, b, c]: Local) -> Point {
        let front = self.on_face(0, a, b);
        if c.abs() <= SAME {
            return front;
        }
        let side = self.on_face(2, c, b);
        if a.abs() <= SAME {
            return side;
        }
        let found = meet([front, self.vanishing(2)], [side, self.vanishing(0)]).unwrap_or(front);
        self.lines.push([front, found]);
        self.lines.push([side, found]);
        found
    }

    /// A square's sides, drawn.
    fn outline(&mut self, corners: &[Point; 4]) {
        for k in 0..4 {
            self.lines.push([corners[k], corners[(k + 1) % 4]]);
        }
    }
}

/// The upright through `point` from `quad`'s bottom edge to its top edge.
fn through(point: Point, [b0, b1, t1, t0]: [Point; 4]) -> [Point; 2] {
    let up = [point, [point[0], point[1] - 1.0]];
    let bottom = meet(up, [b0, b1]).unwrap_or(point);
    let top = meet(up, [t0, t1]).unwrap_or(point);
    [bottom, top]
}
