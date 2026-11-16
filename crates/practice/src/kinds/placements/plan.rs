//! A placement's plan (S153, V大's 分布 of weeks 7 and 8): the ground seen from above, `ground[0]`
//! wide along its near edge (to the right) and `ground[1]` deep (along its left edge, away), and
//! the pieces standing on it, each a footprint and a height, in units: the upright's height, the
//! plan's unit. The ground is divided by its grid of quarters (eighths at 稳, `cells` a side), as
//! its diagonals divide it: a box stands on whole cells; at 熟练 one square is turned in its cell
//! about its middle and one cylinder's circle sits in a square in the middle of its cell.
//!
//! No two pieces share a column of the grid, so the front view under the plan shows each one's
//! height straight under its footprint; pieces in columns side by side keep a row between them,
//! so no two touch; the heights are not all one.

use crate::kinds::placements::placement::Placement;
use crate::maths::path::Point;
use crate::maths::random::Random;

/// Places tried for a piece before the plan is laid again.
const TRIES: usize = 40;
/// A turned square's turn either way, degrees; a turned square's and a cylinder's square's share
/// of their cell's shorter side (the turned one's corners reaching that far across it).
const TURN: [f64; 2] = [22.0, 40.0];
const FILLS: f64 = 0.9;
/// How often a box is one cell wide (else two, where the level allows) and one deep.
const NARROW: f64 = 0.6;
const SHALLOW: f64 = 0.6;

/// Where a piece stands on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Footprint {
    /// A box on whole cells: its columns and rows, each from and to (the last not held).
    Cells {
        columns: [usize; 2],
        rows: [usize; 2],
    },
    /// A square `side` long turned `turn` radians about its cell's middle.
    Turned {
        cell: [usize; 2],
        side: f64,
        turn: f64,
    },
    /// A cylinder: its circle in a square `side` long in the middle of its cell.
    Round { cell: [usize; 2], side: f64 },
}

impl Footprint {
    /// Its columns and rows of the grid, each from and to (the last not held).
    #[must_use]
    pub const fn cells(&self) -> [[usize; 2]; 2] {
        match *self {
            Self::Cells { columns, rows } => [columns, rows],
            Self::Turned { cell, .. } | Self::Round { cell, .. } => {
                [[cell[0], cell[0] + 1], [cell[1], cell[1] + 1]]
            }
        }
    }
}

/// A piece on the ground: its footprint and its height in units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub footprint: Footprint,
    pub height: f64,
}

/// The ground and its pieces, seen from above.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Its width along the near edge and its depth, in units.
    pub ground: [f64; 2],
    /// Its grid's cells a side: 4 (halves and quarters) or 8 (eighths too).
    pub cells: usize,
    pub pieces: Vec<Piece>,
}

/// What a level lays: how many pieces, the grid, the heights, the widest box in cells, the
/// ground's width, and whether one piece is turned and one round.
struct Laying {
    pieces: usize,
    cells: usize,
    heights: &'static [f64],
    widest: usize,
    width: [f64; 2],
    turned_and_round: bool,
}

/// The shapes a piece is laid as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Box,
    Turned,
    Round,
}

const HALVES: [f64; 2] = [0.5, 1.0];
const THIRDS: [f64; 4] = [1.0 / 3.0, 2.0 / 3.0, 1.0, 4.0 / 3.0];
const HALVES_UP: [f64; 3] = [0.5, 1.0, 1.5];

const fn laying(placement: Placement) -> Laying {
    let quarters = Laying {
        pieces: 2,
        cells: 4,
        heights: &HALVES,
        widest: 2,
        width: [3.8, 5.0],
        turned_and_round: false,
    };
    match placement {
        Placement::TwoBoxes => quarters,
        Placement::ThreeBoxes => Laying {
            pieces: 3,
            ..quarters
        },
        Placement::Turned => Laying {
            pieces: 4,
            heights: &THIRDS,
            widest: 1,
            turned_and_round: true,
            ..quarters
        },
        Placement::Above => Laying {
            pieces: 5,
            cells: 8,
            heights: &HALVES_UP,
            width: [4.6, 6.0],
            ..quarters
        },
    }
}

impl Plan {
    /// A plan of `placement`'s laid from `random`; none when its pieces found no room.
    pub(crate) fn laid(placement: Placement, random: &mut Random) -> Option<Self> {
        let laying = laying(placement);
        let width = random.range(laying.width[0], laying.width[1]);
        let ground = [width, width * random.range(0.62, 0.95)];
        let mut plan = Self {
            ground,
            cells: laying.cells,
            pieces: Vec::new(),
        };
        let mut shapes = vec![Shape::Box; laying.pieces];
        if laying.turned_and_round {
            shapes[1] = Shape::Turned;
            shapes[2] = Shape::Round;
        }
        for shape in shapes {
            let piece = (0..TRIES).find_map(|_| {
                let piece = plan.piece(shape, &laying, random);
                plan.pieces
                    .iter()
                    .all(|other| apart(&piece, other))
                    .then_some(piece)
            })?;
            plan.pieces.push(piece);
        }
        let first = plan.pieces[0].height;
        let varied = plan.pieces.iter().any(|piece| piece.height != first);
        varied.then_some(plan)
    }

    /// A piece of `shape` laid anywhere on the grid from `random`.
    fn piece(&self, shape: Shape, laying: &Laying, random: &mut Random) -> Piece {
        let cells = self.cells;
        let wide = if laying.widest > 1 && random.unit() >= NARROW {
            2
        } else {
            1
        };
        let deep = if shape == Shape::Box && random.unit() >= SHALLOW {
            2
        } else {
            1
        };
        let wide = if shape == Shape::Box { wide } else { 1 };
        let pick = |random: &mut Random, span: usize| {
            ((random.unit() * (cells + 1 - span) as f64) as usize).min(cells - span)
        };
        let (column, row) = (pick(random, wide), pick(random, deep));
        let cell = self.cell();
        let shorter = cell[0].min(cell[1]);
        let footprint = match shape {
            Shape::Box => Footprint::Cells {
                columns: [column, column + wide],
                rows: [row, row + deep],
            },
            Shape::Turned => {
                let turn = random.sign() * random.range(TURN[0], TURN[1]).to_radians();
                let reach = turn.cos().abs() + turn.sin().abs();
                Footprint::Turned {
                    cell: [column, row],
                    side: FILLS * shorter / reach,
                    turn,
                }
            }
            Shape::Round => Footprint::Round {
                cell: [column, row],
                side: FILLS * shorter,
            },
        };
        let heights = laying.heights;
        let height =
            heights[((random.unit() * heights.len() as f64) as usize).min(heights.len() - 1)];
        Piece { footprint, height }
    }

    /// A cell's width and depth.
    #[must_use]
    pub fn cell(&self) -> [f64; 2] {
        self.ground.map(|side| side / self.cells as f64)
    }

    /// `piece`'s footprint's corners on the plan (units from the near corner: along the near
    /// edge, then away), in order round counter-clockwise from the plan's above; a cylinder's
    /// those of its square.
    #[must_use]
    pub fn corners(&self, piece: &Piece) -> [Point; 4] {
        let [w, d] = self.cell();
        let middle = |cell: [usize; 2]| [(cell[0] as f64 + 0.5) * w, (cell[1] as f64 + 0.5) * d];
        match piece.footprint {
            Footprint::Cells { columns, rows } => {
                let [u0, u1] = columns.map(|c| c as f64 * w);
                let [v0, v1] = rows.map(|r| r as f64 * d);
                [[u0, v0], [u1, v0], [u1, v1], [u0, v1]]
            }
            Footprint::Turned { cell, side, turn } => {
                let c = middle(cell);
                let (sin, cos) = turn.sin_cos();
                let a = [cos * side / 2.0, sin * side / 2.0];
                let b = [-sin * side / 2.0, cos * side / 2.0];
                [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
                    .map(|[i, j]| [c[0] + i * a[0] + j * b[0], c[1] + i * a[1] + j * b[1]])
            }
            Footprint::Round { cell, side } => {
                let c = middle(cell);
                let h = side / 2.0;
                [
                    [c[0] - h, c[1] - h],
                    [c[0] + h, c[1] - h],
                    [c[0] + h, c[1] + h],
                    [c[0] - h, c[1] + h],
                ]
            }
        }
    }

    /// How far `piece` reaches along the near edge, least and most: its front view's sides.
    #[must_use]
    pub fn span(&self, piece: &Piece) -> [f64; 2] {
        let corners = self.corners(piece);
        let least = corners.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let most = corners
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        [least, most]
    }
}

/// Whether `a` and `b` keep apart: no column shared, and a row between them where their columns
/// meet.
fn apart(a: &Piece, b: &Piece) -> bool {
    let [[ac0, ac1], [ar0, ar1]] = a.footprint.cells();
    let [[bc0, bc1], [br0, br1]] = b.footprint.cells();
    let columns = ac1 <= bc0 || bc1 <= ac0;
    let side_by_side = ac1 == bc0 || bc1 == ac0;
    let rows = ar1 < br0 || br1 < ar0;
    columns && (!side_by_side || rows)
}
