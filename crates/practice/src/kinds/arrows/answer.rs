//! An organic arrow's answer (S156l): the ribbon's two edges as the eye sees them, each sample seen
//! or hidden by the ribbon itself (the ray from the eye against its facets, exact for the sampled
//! ribbon), and its folds: where it turns edge-on to the eye, the outline across it (the line a
//! bend folds along; where a half turn crosses its edges). A fold is where the ribbon's normal lies
//! square to the ray: over the ribbon `S(s, u) = c + u n`, its normal `S_s × S_u` is `b + u n' × n`
//! and `S · (S_s × S_u)` is `c · b + u c · (n' × n)` (the eye at the origin, `s` along the length),
//! straight across at each sample; its zeros are traced by marching squares. The width laid across
//! at even steps along it (看画法) too.

use std::collections::HashMap;
use std::ops::Range;

use crate::kinds::arrows::ribbon::{Ribbon, SAMPLES, View, image};
use crate::kinds::solids::solid_answer::Stretch;
use crate::maths::path::Point;
use crate::maths::vector::{Vector, cross, dot, sub, times};

/// Facets a chunk, whose images' bounds a ray's point is tried against first.
const CHUNK: usize = 24;
/// A facet hides a point only where the ray meets it short of the point by more than this share
/// of the way: the sampled ribbon's chords and twisted quads lie that close to the ribbon.
const DEPTH: f64 = 2e-3;
/// The segments either side of a point's own that are never said to hide it.
const OWN: usize = 2;
/// Strips across the ribbon its folds are traced over; points across a width laid at 看画法.
const ACROSS: usize = 8;
/// The widths laid at even steps along the ribbon (看画法), the head's base the first.
const WIDTHS: usize = 12;

/// The ribbon as the eye sees it, in frame units.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowAnswer {
    /// Each edge at every sample: the first half a width along the normal, the second against it.
    pub edges: [Vec<Point>; 2],
    /// Whether each edge's sample is seen.
    pub seen: [Vec<bool>; 2],
    /// Each edge in stretches, seen or hidden.
    pub stretches: [Vec<Stretch>; 2],
    /// Where the ribbon folds: its outline across it, each fold in stretches.
    pub folds: Vec<Vec<Stretch>>,
    /// The width across at even steps along the ribbon (看画法), in stretches.
    pub widths: Vec<Stretch>,
    /// Each fold's place along the ribbon at its points (a sample's index and a share of the way
    /// to the next).
    pub(crate) placed: Vec<Vec<f64>>,
    /// Each edge's hidden samples, each with the segment between samples that hides it.
    pub(crate) behind: [Vec<(usize, usize)>; 2],
}

/// `ribbon` seen through `view`, its arrowhead `head` (world) hiding what lies behind it.
pub(crate) fn answered(ribbon: &Ribbon, view: &View, head: [Vector; 3]) -> ArrowAnswer {
    let mesh = Mesh::of(ribbon, head);
    let half = ribbon.half;
    let edge = |u: f64| -> (Vec<Point>, Vec<bool>, Vec<(usize, usize)>) {
        let mut behind = Vec::new();
        let (points, seen) = (0..SAMPLES)
            .map(|i| {
                let p = ribbon.across(i, u);
                let hiding = mesh.hiding(p, own(i as f64));
                if let Some(segment) = hiding {
                    behind.push((i, segment));
                }
                (view.frame(p), hiding.is_none())
            })
            .unzip();
        (points, seen, behind)
    };
    let (first, first_seen, first_behind) = edge(half);
    let (second, second_seen, second_behind) = edge(-half);
    let behind = [first_behind, second_behind];
    let stretches = [
        stretched(&first, &first_seen),
        stretched(&second, &second_seen),
    ];
    let traced = traced(ribbon);
    let folds = (traced.iter())
        .map(|fold| {
            let (points, seen): (Vec<Point>, Vec<bool>) = (fold.iter())
                .map(|&[at, u]| {
                    let p = ribbon.between(at, u);
                    (view.frame(p), mesh.hiding(p, own(at)).is_none())
                })
                .unzip();
            stretched(&points, &seen)
        })
        .collect();
    let placed = (traced.iter())
        .map(|fold| fold.iter().map(|[at, _]| *at).collect())
        .collect();
    let widths = (1..=WIDTHS)
        .flat_map(|k| {
            let i = k * (SAMPLES - 1) / WIDTHS;
            let (points, seen): (Vec<Point>, Vec<bool>) = (0..=ACROSS)
                .map(|m| {
                    let u = half * (2.0 * m as f64 / ACROSS as f64 - 1.0);
                    let p = ribbon.across(i, u);
                    (view.frame(p), mesh.hiding(p, own(i as f64)).is_none())
                })
                .unzip();
            stretched(&points, &seen)
        })
        .collect();
    ArrowAnswer {
        edges: [first, second],
        seen: [first_seen, second_seen],
        stretches,
        folds,
        widths,
        placed,
        behind,
    }
}

/// The segments a point at the place `at` along the ribbon never counts as hiding it: its own
/// and `OWN` either side.
fn own(at: f64) -> Range<usize> {
    let i = at.floor().max(0.0) as usize;
    i.saturating_sub(OWN)..i + OWN
}

/// `points` in stretches by `seen`, each stretch from and to the middle between two points where
/// seeing changes, so they join.
fn stretched(points: &[Point], seen: &[bool]) -> Vec<Stretch> {
    let mut stretches: Vec<Stretch> = Vec::new();
    for (k, (p, is_seen)) in points.iter().zip(seen).enumerate() {
        match stretches.last_mut() {
            Some(last) if last.hidden != *is_seen => last.points.push(*p),
            _ => {
                let mut begun = Vec::with_capacity(2);
                if let Some(last) = stretches.last_mut() {
                    let before = points[k - 1];
                    let middle = [(before[0] + p[0]) / 2.0, (before[1] + p[1]) / 2.0];
                    last.points.push(middle);
                    begun.push(middle);
                }
                begun.push(*p);
                stretches.push(Stretch {
                    points: begun,
                    hidden: !is_seen,
                });
            }
        }
    }
    stretches
}

/// The ribbon as facets (two a segment between samples, and the arrowhead), in chunks whose
/// images' bounds are known: a ray is tried against the facets its point's image lies in.
struct Mesh {
    facets: Vec<Facet>,
    chunks: Vec<(Range<usize>, [Point; 2])>,
}

/// A triangle: a corner, its two sides from it, the segment between samples it belongs to (the
/// arrowhead the first) and its image's bounds.
struct Facet {
    corner: Vector,
    sides: [Vector; 2],
    segment: usize,
    bounds: [Point; 2],
}

impl Mesh {
    fn of(ribbon: &Ribbon, head: [Vector; 3]) -> Self {
        let half = ribbon.half;
        let mut facets = Vec::with_capacity(2 * SAMPLES);
        for i in 0..SAMPLES - 1 {
            let [a, b] = [ribbon.across(i, half), ribbon.across(i, -half)];
            let [c, d] = [ribbon.across(i + 1, -half), ribbon.across(i + 1, half)];
            facets.push(facet([a, b, c], i));
            facets.push(facet([a, c, d], i));
        }
        facets.push(facet(head, 0));
        let chunks = (0..facets.len())
            .step_by(CHUNK)
            .map(|from| {
                let range = from..(from + CHUNK).min(facets.len());
                let corners = facets[range.clone()].iter().flat_map(|f| f.bounds);
                (range, bounds(corners))
            })
            .collect();
        Self { facets, chunks }
    }

    /// The segment that hides `p` from the eye: of the first facet the ray to it meets short of
    /// it, but those of the segments `own`; none where it is seen.
    fn hiding(&self, p: Vector, own: Range<usize>) -> Option<usize> {
        let at = image(p);
        let within = |[low, high]: [Point; 2]| {
            (low[0]..=high[0]).contains(&at[0]) && (low[1]..=high[1]).contains(&at[1])
        };
        (self.chunks.iter())
            .filter(|(_, chunk)| within(*chunk))
            .flat_map(|(range, _)| &self.facets[range.clone()])
            .find(|facet| !own.contains(&facet.segment) && within(facet.bounds) && meets(facet, p))
            .map(|facet| facet.segment)
    }
}

/// The facet with these corners, of `segment`.
fn facet([a, b, c]: [Vector; 3], segment: usize) -> Facet {
    Facet {
        corner: a,
        sides: [sub(b, a), sub(c, a)],
        segment,
        bounds: bounds([a, b, c].into_iter().map(image)),
    }
}

/// The least and the greatest of `points` each way.
fn bounds(points: impl Iterator<Item = Point>) -> [Point; 2] {
    points.fold(
        [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]],
        |[low, high], p| {
            [
                [low[0].min(p[0]), low[1].min(p[1])],
                [high[0].max(p[0]), high[1].max(p[1])],
            ]
        },
    )
}

/// Whether the ray from the eye to `p` meets `facet` short of `p` by more than `DEPTH` of the way
/// (Möller and Trumbore).
fn meets(facet: &Facet, p: Vector) -> bool {
    let [first, second] = facet.sides;
    let across = cross(p, second);
    let det = dot(first, across);
    if det.abs() < 1e-14 {
        return false;
    }
    let from = times(facet.corner, -1.0);
    let u = dot(from, across) / det;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let turned = cross(from, first);
    let v = dot(p, turned) / det;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = dot(second, turned) / det;
    t > 0.0 && t < 1.0 - DEPTH
}

/// Where a crossing of the grid lies: `Along(i, j)` between samples `i` and `i + 1` at row `j`
/// across, `Across(i, j)` at sample `i` between rows `j` and `j + 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Crossed {
    Along(usize, usize),
    Across(usize, usize),
}

/// The ribbon's folds: the zeros of `S · (S_s × S_u)` over its samples and `ACROSS` rows across,
/// each a line of places (a sample's index and a share of the way on, and how far across).
fn traced(ribbon: &Ribbon) -> Vec<Vec<[f64; 2]>> {
    let half = ribbon.half;
    let row = |j: usize| half * (2.0 * j as f64 / ACROSS as f64 - 1.0);
    let [f, g] = facing(ribbon);
    let value = |i: usize, j: usize| f[i] + row(j) * g[i];
    let mut segments: Vec<[Crossed; 2]> = Vec::new();
    for i in 0..SAMPLES - 1 {
        for j in 0..ACROSS {
            let corners = [
                value(i, j),
                value(i + 1, j),
                value(i + 1, j + 1),
                value(i, j + 1),
            ];
            segments.extend(cell(i, j, corners));
        }
    }
    let place = |crossed: Crossed| match crossed {
        Crossed::Along(i, j) => {
            let (a, b) = (value(i, j), value(i + 1, j));
            [i as f64 + a / (a - b), row(j)]
        }
        Crossed::Across(i, j) => {
            let (a, b) = (value(i, j), value(i, j + 1));
            [i as f64, row(j) + (row(j + 1) - row(j)) * a / (a - b)]
        }
    };
    chained(&segments)
        .into_iter()
        .map(|line| line.into_iter().map(place).collect())
        .collect()
}

/// At each sample, `c · b` and `c · (n' × n)`: how `S · (S_s × S_u)` lies across the ribbon there
/// (`n'` the normal's change along the length, by its neighbours).
fn facing(ribbon: &Ribbon) -> [Vec<f64>; 2] {
    let step = ribbon.path.length() / (SAMPLES - 1) as f64;
    let samples = &ribbon.samples;
    let f = samples.iter().map(|s| dot(s.at, s.binormal)).collect();
    let g = (0..SAMPLES)
        .map(|i| {
            let (before, after) = (i.saturating_sub(1), (i + 1).min(SAMPLES - 1));
            let changed = sub(samples[after].normal, samples[before].normal);
            let turned = times(changed, 1.0 / (step * (after - before) as f64));
            dot(samples[i].at, cross(turned, samples[i].normal))
        })
        .collect();
    [f, g]
}

/// The pieces of zero line in the cell from sample `i`, row `j` (its corners' values in order
/// round from there: `i, j`, `i + 1, j`, `i + 1, j + 1`, `i, j + 1`).
fn cell(i: usize, j: usize, [a, b, c, d]: [f64; 4]) -> Vec<[Crossed; 2]> {
    let differ = |x: f64, y: f64| (x >= 0.0) != (y >= 0.0);
    let bottom = differ(a, b).then_some(Crossed::Along(i, j));
    let right = differ(b, c).then_some(Crossed::Across(i + 1, j));
    let top = differ(d, c).then_some(Crossed::Along(i, j + 1));
    let left = differ(a, d).then_some(Crossed::Across(i, j));
    match (bottom, right, top, left) {
        (Some(bottom), Some(right), Some(top), Some(left)) => {
            let middle = (a + b + c + d) / 4.0;
            if (middle >= 0.0) == (a >= 0.0) {
                vec![[bottom, right], [left, top]]
            } else {
                vec![[bottom, left], [top, right]]
            }
        }
        _ => {
            let crossed: Vec<Crossed> = [bottom, right, top, left].into_iter().flatten().collect();
            match crossed[..] {
                [from, to] => vec![[from, to]],
                _ => Vec::new(),
            }
        }
    }
}

/// `segments` joined where they share a crossing into lines, those ending on the grid's edge
/// first, then closed ones.
fn chained(segments: &[[Crossed; 2]]) -> Vec<Vec<Crossed>> {
    let mut at: HashMap<Crossed, Vec<usize>> = HashMap::new();
    for (k, pair) in segments.iter().enumerate() {
        for end in pair {
            at.entry(*end).or_default().push(k);
        }
    }
    let mut used = vec![false; segments.len()];
    let mut lines = Vec::new();
    let ends: Vec<Crossed> = (segments.iter().flatten())
        .filter(|end| at.get(end).is_some_and(|k| k.len() == 1))
        .copied()
        .collect();
    let starts = ends.into_iter().chain(segments.iter().map(|pair| pair[0]));
    for start in starts {
        let mut line = vec![start];
        let mut here = start;
        while let Some(&k) = (at.get(&here).into_iter().flatten()).find(|&&k| !used[k]) {
            used[k] = true;
            let [a, b] = segments[k];
            here = if a == here { b } else { a };
            line.push(here);
        }
        if line.len() > 1 {
            lines.push(line);
        }
    }
    lines
}
