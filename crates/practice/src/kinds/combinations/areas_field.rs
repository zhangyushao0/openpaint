//! A colour combination's areas as a field over the frame (`areas.rs`): which area a point lies
//! in, the area across its nearest edge and how far that edge is, so a picture can be drawn with
//! hard or soft edges and the map read from the same shapes. Three arrangements after the crops
//! V大 picks: bands across the frame (bark, strata, a rusted plate's layers), patches round a few
//! centres (rust, lichen, an abstract painting's blots) and blocks splitting the frame (an
//! abstract painting's fields). A slow warp lets every edge wander as a hand or weather lays it.

use std::f64::consts::TAU;

use crate::path::Point;
use crate::random::Random;

/// A smooth displacement of the frame: two slow waves along each axis.
#[derive(Debug, Clone, PartialEq)]
struct Warp {
    /// Each wave: its frequency (cycles per frame unit, a vector), its phase (radians) and the
    /// axis it moves.
    waves: Vec<(Point, f64, usize)>,
    /// How far a point moves at most along an axis, in frame units.
    amplitude: f64,
}

impl Warp {
    fn new(random: &mut Random, amplitude: f64) -> Self {
        let waves = (0..4)
            .map(|k| {
                let angle = random.range(0.0, TAU);
                let cycles = random.range(0.8, 2.0);
                let frequency = [angle.cos() * cycles, angle.sin() * cycles];
                (frequency, random.range(0.0, TAU), k % 2)
            })
            .collect();
        Self { waves, amplitude }
    }

    fn at(&self, p: Point) -> Point {
        let mut moved = p;
        for (frequency, phase, axis) in &self.waves {
            let wave = (TAU * (frequency[0] * p[0] + frequency[1] * p[1]) + phase).sin();
            moved[*axis] += self.amplitude * wave / 2.0;
        }
        moved
    }
}

/// The areas' shapes before the warp.
#[derive(Debug, Clone, PartialEq)]
enum Shapes {
    /// Bands across `normal` (a unit vector), cut at `cuts` along it from the frame's middle,
    /// lowest first: one cut fewer than bands.
    Bands { normal: Point, cuts: Vec<f64> },
    /// Patches round `sites`, a larger `weights` a larger patch (a power diagram).
    Patches {
        sites: Vec<Point>,
        weights: Vec<f64>,
    },
    /// Rectangles splitting the frame: left, top, right, bottom.
    Blocks(Vec<[f64; 4]>),
}

/// Where a point lies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct At {
    pub(crate) area: usize,
    /// The area across the nearest edge (the point's own where there is none).
    pub(crate) across: usize,
    /// How far that edge is, in frame units.
    pub(crate) edge: f64,
}

/// A colour combination's areas over the frame.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Field {
    shapes: Shapes,
    warp: Warp,
}

impl Field {
    /// Bands (`0`), patches (`1`) or blocks (`2`), from `random`.
    pub(crate) fn generate(arrangement: usize, random: &mut Random) -> Self {
        let (shapes, amplitude) = match arrangement {
            0 => (bands(random), 0.05),
            1 => (patches(random), 0.06),
            _ => (blocks(random), 0.015),
        };
        Self {
            shapes,
            warp: Warp::new(random, amplitude),
        }
    }

    /// How many areas there are.
    pub(crate) fn count(&self) -> usize {
        match &self.shapes {
            Shapes::Bands { cuts, .. } => cuts.len() + 1,
            Shapes::Patches { sites, .. } => sites.len(),
            Shapes::Blocks(rects) => rects.len(),
        }
    }

    /// Where the frame point `p` lies.
    pub(crate) fn at(&self, p: Point) -> At {
        let q = self.warp.at(p);
        match &self.shapes {
            Shapes::Bands { normal, cuts } => in_bands(*normal, cuts, q),
            Shapes::Patches { sites, weights } => in_patches(sites, weights, q),
            Shapes::Blocks(rects) => in_blocks(rects, q),
        }
    }
}

/// Four or five bands, mostly across the frame (now and then up it), each at least a tenth of it.
fn bands(random: &mut Random) -> Shapes {
    let count = 4 + usize::from(random.unit() < 0.5);
    let upright = random.unit() < 0.3;
    let tilt = random.range(-35.0f64, 35.0).to_radians();
    let angle = if upright { tilt } else { TAU / 4.0 + tilt };
    let normal = [angle.cos(), angle.sin()];
    // The frame's extent along the normal, from its middle.
    let reach = (normal[0].abs() + normal[1].abs()) / 2.0;
    let widths: Vec<f64> = (0..count).map(|_| random.range(0.6, 1.4)).collect();
    let total: f64 = widths.iter().sum();
    let mut at = -reach;
    let cuts = widths[..count - 1]
        .iter()
        .map(|width| {
            at += 2.0 * reach * width / total;
            at
        })
        .collect();
    Shapes::Bands { normal, cuts }
}

/// Four to six patches round sites at least 0.22 apart, well inside the frame.
fn patches(random: &mut Random) -> Shapes {
    let count = 4 + (random.unit() * 3.0) as usize;
    let mut sites: Vec<Point> = Vec::with_capacity(count);
    while sites.len() < count {
        let site = [random.range(0.1, 0.9), random.range(0.1, 0.9)];
        let apart = |other: &Point| (site[0] - other[0]).hypot(site[1] - other[1]) >= 0.22;
        if sites.iter().all(apart) {
            sites.push(site);
        }
    }
    let weights = (0..count).map(|_| random.range(-0.02, 0.02)).collect();
    Shapes::Patches { sites, weights }
}

/// Four or five rectangles, the largest split across its longer side again and again, none
/// narrower than 0.15.
fn blocks(random: &mut Random) -> Shapes {
    let count = 4 + usize::from(random.unit() < 0.5);
    let mut rects = vec![[0.0, 0.0, 1.0, 1.0]];
    while rects.len() < count {
        let area = |r: &[f64; 4]| (r[2] - r[0]) * (r[3] - r[1]);
        let largest = (0..rects.len())
            .max_by(|a, b| area(&rects[*a]).total_cmp(&area(&rects[*b])))
            .unwrap_or(0);
        let [left, top, right, bottom] = rects[largest];
        let across = right - left >= bottom - top;
        let (low, high) = if across { (left, right) } else { (top, bottom) };
        let cut = low + (high - low) * random.range(0.35, 0.65);
        if cut - low < 0.15 || high - cut < 0.15 {
            // A sliver: fewer blocks, which the generator refuses and lays anew.
            break;
        }
        let (first, second) = if across {
            ([left, top, cut, bottom], [cut, top, right, bottom])
        } else {
            ([left, top, right, cut], [left, cut, right, bottom])
        };
        rects[largest] = first;
        rects.push(second);
    }
    Shapes::Blocks(rects)
}

fn in_bands(normal: Point, cuts: &[f64], q: Point) -> At {
    let t = normal[0] * (q[0] - 0.5) + normal[1] * (q[1] - 0.5);
    let area = cuts.iter().filter(|cut| t >= **cut).count();
    let below = area.checked_sub(1).map(|cut| (t - cuts[cut], area - 1));
    let above = cuts.get(area).map(|cut| (cut - t, area + 1));
    let (edge, across) = match (below, above) {
        (Some(below), Some(above)) => {
            if below.0 < above.0 {
                below
            } else {
                above
            }
        }
        (Some(one), None) | (None, Some(one)) => one,
        (None, None) => (f64::INFINITY, area),
    };
    At { area, across, edge }
}

fn in_patches(sites: &[Point], weights: &[f64], q: Point) -> At {
    let power = |i: usize| (q[0] - sites[i][0]).powi(2) + (q[1] - sites[i][1]).powi(2) - weights[i];
    let area = (0..sites.len())
        .min_by(|a, b| power(*a).total_cmp(&power(*b)))
        .unwrap_or(0);
    let mut nearest = (f64::INFINITY, area);
    for other in (0..sites.len()).filter(|other| *other != area) {
        let apart = (sites[area][0] - sites[other][0]).hypot(sites[area][1] - sites[other][1]);
        // The distance to the bisector between the two (for their powers).
        let edge = (power(other) - power(area)) / (2.0 * apart.max(1e-9));
        if edge < nearest.0 {
            nearest = (edge, other);
        }
    }
    At {
        area,
        across: nearest.1,
        edge: nearest.0,
    }
}

fn in_blocks(rects: &[[f64; 4]], q: Point) -> At {
    let q = q.map(|v| v.clamp(0.0, 1.0 - 1e-9));
    let holding = |p: Point| {
        (rects.iter())
            .position(|r| p[0] >= r[0] && p[0] < r[2] && p[1] >= r[1] && p[1] < r[3])
            .unwrap_or(0)
    };
    let area = holding(q);
    let [left, top, right, bottom] = rects[area];
    // Each side inside the frame: how far, and a point just across it.
    let sides = [
        (left, q[0] - left, [left - 1e-6, q[1]]),
        (right, right - q[0], [right + 1e-6, q[1]]),
        (top, q[1] - top, [q[0], top - 1e-6]),
        (bottom, bottom - q[1], [q[0], bottom + 1e-6]),
    ];
    let nearest = (sides.iter())
        .filter(|(at, _, _)| *at > 0.0 && *at < 1.0)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    match nearest {
        Some((_, edge, across)) => At {
            area,
            across: holding(*across),
            edge: *edge,
        },
        None => At {
            area,
            across: area,
            edge: f64::INFINITY,
        },
    }
}
