//! The silhouette's contours (`outline.rs`): the grid's mask smoothed a little (a binomial blur,
//! so the contour runs between pixels rather than along their stairs), traced at half by
//! marching squares into closed polylines, and thinned by Douglas and Peucker's rule.

use crate::maths::path::{Point, distance};

/// The mask, padded by one ground pixel all round and blurred by [1 2 1] across and down.
struct Field {
    width: usize,
    height: usize,
    values: Vec<f32>,
}

impl Field {
    fn of([width, height]: [usize; 2], inside: &[bool]) -> Self {
        let (w, h) = (width + 2, height + 2);
        let mut values = vec![0.0f32; w * h];
        for y in 0..height {
            for x in 0..width {
                if inside[y * width + x] {
                    values[(y + 1) * w + x + 1] = 1.0;
                }
            }
        }
        let mut field = Self {
            width: w,
            height: h,
            values,
        };
        field.blur(true);
        field.blur(false);
        field
    }

    /// One pass of [1 2 1] / 4 along rows (`across`) or columns, the padding kept.
    fn blur(&mut self, across: bool) {
        let (w, h) = (self.width, self.height);
        let before = self.values.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let step = if across { 1 } else { w };
                self.values[i] =
                    0.25 * before[i - step] + 0.5 * before[i] + 0.25 * before[i + step];
            }
        }
    }

    fn at(&self, x: usize, y: usize) -> f32 {
        self.values[y * self.width + x]
    }
}

/// The closed contours of `inside` (`size` grid pixels) in grid units (pixel `(x, y)` covers `x`
/// to `x + 1`), each without its first point repeated.
pub(crate) fn contours(size: [usize; 2], inside: &[bool]) -> Vec<Vec<Point>> {
    let field = Field::of(size, inside);
    let segments = segments(&field);
    link(&segments)
        .into_iter()
        .map(|ids| ids.iter().map(|id| crossing(&field, *id)).collect())
        .collect()
}

/// A crossing of the level on the field's grid: on the edge right of node `(x, y)` (`down`
/// false) or below it, as one number.
type Crossing = u64;

fn crossing_id(field: &Field, x: usize, y: usize, down: bool) -> Crossing {
    2 * (y * field.width + x) as u64 + u64::from(down)
}

/// Where crossing `id` lies, in grid units: along its edge where the field passes a half.
fn crossing(field: &Field, id: Crossing) -> Point {
    let node = (id / 2) as usize;
    let (x, y) = (node % field.width, node / field.width);
    let (nx, ny) = if id % 2 == 1 { (x, y + 1) } else { (x + 1, y) };
    let (a, b) = (field.at(x, y), field.at(nx, ny));
    let t = if (b - a).abs() > f32::EPSILON {
        f64::from(((0.5 - a) / (b - a)).clamp(0.0, 1.0))
    } else {
        0.5
    };
    // Node (x, y) of the padded field is the centre of grid pixel (x - 1, y - 1).
    let [px, py] = [x as f64 - 0.5, y as f64 - 0.5];
    [
        px + t * (nx as f64 - x as f64),
        py + t * (ny as f64 - y as f64),
    ]
}

/// Every cell's pieces of the level line, as pairs of crossings.
fn segments(field: &Field) -> Vec<[Crossing; 2]> {
    let mut found = Vec::new();
    for y in 0..field.height - 1 {
        for x in 0..field.width - 1 {
            let corners = [
                field.at(x, y),
                field.at(x + 1, y),
                field.at(x + 1, y + 1),
                field.at(x, y + 1),
            ];
            let case = corners
                .iter()
                .enumerate()
                .fold(0u8, |case, (bit, v)| case | (u8::from(*v >= 0.5) << bit));
            // The cell's edges: top, right, bottom, left.
            let edges = [
                crossing_id(field, x, y, false),
                crossing_id(field, x + 1, y, true),
                crossing_id(field, x, y + 1, false),
                crossing_id(field, x, y, true),
            ];
            let middle = corners.iter().sum::<f32>() / 4.0 >= 0.5;
            for [a, b] in cell_pieces(case, middle) {
                found.push([edges[a], edges[b]]);
            }
        }
    }
    found
}

/// The pieces a cell of `case` (bit n set: corner n inside; corners top left, top right, bottom
/// right, bottom left) holds, as pairs of its edges (top, right, bottom, left); a saddle joins
/// round its middle when that is inside.
fn cell_pieces(case: u8, middle: bool) -> Vec<[usize; 2]> {
    match case {
        1 | 14 => vec![[3, 0]],
        2 | 13 => vec![[0, 1]],
        3 | 12 => vec![[3, 1]],
        4 | 11 => vec![[1, 2]],
        6 | 9 => vec![[0, 2]],
        7 | 8 => vec![[3, 2]],
        5 if middle => vec![[3, 2], [0, 1]],
        5 => vec![[3, 0], [1, 2]],
        10 if middle => vec![[3, 0], [1, 2]],
        10 => vec![[3, 2], [0, 1]],
        _ => Vec::new(),
    }
}

/// The pieces joined end to end into closed loops, each a list of crossings.
fn link(segments: &[[Crossing; 2]]) -> Vec<Vec<Crossing>> {
    let mut at: std::collections::HashMap<Crossing, Vec<usize>> = std::collections::HashMap::new();
    for (index, [a, b]) in segments.iter().enumerate() {
        at.entry(*a).or_default().push(index);
        at.entry(*b).or_default().push(index);
    }
    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let [first, mut end] = segments[start];
        let mut ids = vec![first];
        while end != first {
            ids.push(end);
            let next = at
                .get(&end)
                .and_then(|pieces| pieces.iter().copied().find(|&piece| !used[piece]));
            let Some(next) = next else {
                break;
            };
            used[next] = true;
            let [a, b] = segments[next];
            end = if a == end { b } else { a };
        }
        if ids.len() >= 3 {
            loops.push(ids);
        }
    }
    loops
}

/// The signed area a closed polyline encloses (positive when its turn is clockwise on screen).
pub(crate) fn area(contour: &[Point]) -> f64 {
    let count = contour.len();
    (0..count)
        .map(|i| {
            let (a, b) = (contour[i], contour[(i + 1) % count]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// The length round a closed polyline.
pub(crate) fn perimeter(contour: &[Point]) -> f64 {
    let count = contour.len();
    (0..count)
        .map(|i| distance(contour[i], contour[(i + 1) % count]))
        .sum()
}

/// A closed polyline thinned to the points it needs within `tolerance` (Douglas and Peucker,
/// from its first point and the point farthest from it).
pub(crate) fn thinned(contour: &[Point], tolerance: f64) -> Vec<Point> {
    if contour.len() < 4 {
        return contour.to_vec();
    }
    let far = (0..contour.len())
        .max_by(|&a, &b| {
            distance(contour[0], contour[a]).total_cmp(&distance(contour[0], contour[b]))
        })
        .unwrap_or(0);
    let mut closed = contour.to_vec();
    closed.push(contour[0]);
    let mut keep = vec![false; closed.len()];
    keep[0] = true;
    keep[far] = true;
    keep[closed.len() - 1] = true;
    let mut spans = vec![(0, far), (far, closed.len() - 1)];
    while let Some((from, to)) = spans.pop() {
        if to <= from + 1 {
            continue;
        }
        let (worst, gap) = (from + 1..to)
            .map(|i| (i, off_segment(closed[i], closed[from], closed[to])))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap_or((from, 0.0));
        if gap > tolerance {
            keep[worst] = true;
            spans.push((from, worst));
            spans.push((worst, to));
        }
    }
    closed.pop();
    (closed.into_iter().zip(keep))
        .filter_map(|(p, kept)| kept.then_some(p))
        .collect()
}

/// How far `p` lies from the segment from `a` to `b`.
pub(crate) fn off_segment(p: Point, a: Point, b: Point) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let squared = d[0] * d[0] + d[1] * d[1];
    if squared <= 0.0 {
        return distance(p, a);
    }
    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / squared).clamp(0.0, 1.0);
    distance(p, [a[0] + t * d[0], a[1] + t * d[1]])
}
