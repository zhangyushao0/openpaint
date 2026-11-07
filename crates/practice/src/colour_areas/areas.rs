//! Colour combinations (V大's week 2, 临摹色彩组合): a small picture of a few colour areas,
//! copied by eye beside it: its overall tone first, then the middle tones, then the hierarchy,
//! never picked from the original. V大 picks a corner of an abstract oil painting or of a rusted
//! old thing, never a concrete object; since S101 the pictures are such crops of real pictures,
//! cut and answered by the pack builder (`crop.rs`, `areas_found.rs`). S82's generated pictures
//! (`areas_generated.rs`) stay for the records made of them, the How's example and the tests.
//!
//! This is the answer either way: a map of the picture's areas on a grid of cells, `GRID` along
//! its longer side: which area each cell lies in, each area's colour, which areas meet, and each
//! area's core, the cells deep enough inside it that neither its soft edges nor a copy's slightly
//! misplaced ones reach them. The copy is read there (`areas_read.rs`), its edges drawn from it
//! (`areas_edges.rs`). Points are shares of the picture's width and height (0 to 1).

use crate::maths::colour::{Rgb8, delta_e};

/// The map's cells along the picture's longer side.
pub const GRID: usize = 128;
/// A contrast between neighbours this small or smaller is weak: under three just-noticeable
/// steps (Oklab 0.02 each).
pub const WEAK: f64 = 0.06;
/// One this large or larger is strong: more than seven such steps, about a step and a half of
/// Munsell value.
pub const STRONG: f64 = 0.15;
/// The most areas a combination holds.
pub const MOST_AREAS: usize = 7;
/// The least share of the picture an area holds.
pub(crate) const LEAST_SHARE: f64 = 0.05;
/// Cells a core lies inside its area's edges, beyond any soft edge's reach: about 19 canvas
/// pixels on a 400-pixel frame, more than a copy's edge is commonly off.
pub(crate) const CORE: usize = 6;
/// The least cells a core holds.
pub(crate) const LEAST_CORE: usize = 24;
/// Cell edges two areas share to count as neighbours.
const SHARED: usize = 4;

/// A colour combination's answer: its areas as a map of cells, their colours and which meet.
#[derive(Debug, Clone, PartialEq)]
pub struct Areas {
    /// Cells across and down: `GRID` along the picture's longer side.
    pub size: [usize; 2],
    /// Each area's colour.
    pub colours: Vec<Rgb8>,
    /// Each area's edge's half width as a share of the picture's longer side: a generated
    /// picture's soft edges; 0 for a hard edge and for every crop's (its edges read as they are).
    pub soft: Vec<f64>,
    /// The areas that meet, each pair once, the lower index first.
    pub neighbours: Vec<(usize, usize)>,
    /// The area of each cell, row by row.
    map: Vec<u8>,
    /// How deep each cell lies in its area, in cells (1 at an edge), capped at 255.
    depth: Vec<u8>,
}

impl Areas {
    /// The answer whose map of `size` cells is `map` (each cell an area's index) and whose areas
    /// are `colours`, every edge hard; none unless each side is 1 to `GRID` cells, the map holds
    /// as many cells, every cell names one of the 1 to `MOST_AREAS` colours and every area holds
    /// a cell: what a pack's head is read into.
    #[must_use]
    pub fn of_map(size: [usize; 2], map: Vec<u8>, colours: Vec<Rgb8>) -> Option<Self> {
        let count = colours.len();
        let sides = size.iter().all(|side| (1..=GRID).contains(side));
        let held = |area: usize| map.iter().any(|a| usize::from(*a) == area);
        let valid = sides
            && (1..=MOST_AREAS).contains(&count)
            && map.len() == size[0] * size[1]
            && map.iter().all(|a| usize::from(*a) < count)
            && (0..count).all(held);
        valid.then(|| Self::laid(size, map, colours, vec![0.0; count]))
    }

    /// The answer of `map` (`size` cells, its areas' indices checked) with `colours` and `soft`
    /// edges; its neighbours and depths derived.
    pub(crate) fn laid(size: [usize; 2], map: Vec<u8>, colours: Vec<Rgb8>, soft: Vec<f64>) -> Self {
        let count = colours.len();
        Self {
            size,
            neighbours: neighbours(&map, size, count),
            depth: depth(&map, size),
            colours,
            soft,
            map,
        }
    }

    /// How many areas there are.
    #[must_use]
    pub fn count(&self) -> usize {
        self.colours.len()
    }

    /// The area of each cell, row by row: what a pack keeps.
    #[must_use]
    pub fn map(&self) -> &[u8] {
        &self.map
    }

    /// The area the cell `[column, row]` lies in.
    pub(crate) fn cell(&self, [column, row]: [usize; 2]) -> usize {
        let [w, h] = self.size;
        usize::from(self.map[row.min(h - 1) * w + column.min(w - 1)])
    }

    /// How many cells each area holds.
    #[must_use]
    pub fn sizes(&self) -> Vec<usize> {
        let mut sizes = vec![0; self.count()];
        for area in &self.map {
            if let Some(size) = sizes.get_mut(usize::from(*area)) {
                *size += 1;
            }
        }
        sizes
    }

    /// The neighbouring pairs whose contrast is weak (`WEAK` or less).
    #[must_use]
    pub fn weak_pairs(&self) -> Vec<(usize, usize)> {
        (self.neighbours.iter().copied())
            .filter(|(a, b)| delta_e(self.colours[*a], self.colours[*b]) <= WEAK)
            .collect()
    }

    /// How deep `area`'s deepest cell lies, in cells.
    pub(crate) fn deepest(&self, area: usize) -> usize {
        (self.map.iter().zip(&self.depth))
            .filter(|(a, _)| usize::from(**a) == area)
            .map(|(_, depth)| usize::from(*depth))
            .max()
            .unwrap_or(0)
    }

    /// The cells of `area`'s core: deep enough that neither a soft edge of it or of a neighbour
    /// nor a copy's edge a little off reaches them, as indices into the map.
    pub(crate) fn core(&self, area: usize) -> Vec<usize> {
        let reach = (self.neighbours.iter())
            .filter(|(a, b)| *a == area || *b == area)
            .flat_map(|(a, b)| [self.soft[*a], self.soft[*b]])
            .fold(self.soft[area], f64::max);
        let deep = CORE + (reach * GRID as f64).ceil() as usize;
        (0..self.map.len())
            .filter(|cell| {
                usize::from(self.map[*cell]) == area && usize::from(self.depth[*cell]) > deep
            })
            .collect()
    }
}

/// The pairs of areas sharing at least `SHARED` cell edges.
fn neighbours(map: &[u8], [w, h]: [usize; 2], count: usize) -> Vec<(usize, usize)> {
    let mut shared = vec![0usize; count * count];
    for row in 0..h {
        for column in 0..w {
            let here = usize::from(map[row * w + column]);
            let right = (column + 1 < w).then(|| map[row * w + column + 1]);
            let below = (row + 1 < h).then(|| map[(row + 1) * w + column]);
            for there in [right, below].into_iter().flatten().map(usize::from) {
                if there != here && here < count && there < count {
                    shared[here.min(there) * count + here.max(there)] += 1;
                }
            }
        }
    }
    (0..count)
        .flat_map(|a| ((a + 1)..count).map(move |b| (a, b)))
        .filter(|(a, b)| shared[a * count + b] >= SHARED)
        .collect()
}

/// How deep each cell of `map` (`[w, h]` cells, any labels) lies in its area: 1 beside another
/// area's cell (the picture's edge is no edge), one more a step (of eight neighbours) further in,
/// capped at 255.
pub(crate) fn depth<T: PartialEq>(map: &[T], [w, h]: [usize; 2]) -> Vec<u8> {
    let mut depth = vec![u8::MAX; map.len()];
    let mut front = Vec::new();
    let around = |cell: usize| {
        let (row, column) = ((cell / w) as isize, (cell % w) as isize);
        (-1..=1)
            .flat_map(move |dy| (-1..=1).map(move |dx| (row + dy, column + dx)))
            .filter(move |(r, c)| (0..h as isize).contains(r) && (0..w as isize).contains(c))
            .map(move |(r, c)| r as usize * w + c as usize)
    };
    for cell in 0..map.len() {
        if around(cell).any(|other| map[other] != map[cell]) {
            depth[cell] = 1;
            front.push(cell);
        }
    }
    let mut level = 1u8;
    while !front.is_empty() && level < u8::MAX {
        level += 1;
        let mut next = Vec::new();
        for cell in front {
            for other in around(cell) {
                if depth[other] == u8::MAX {
                    depth[other] = level;
                    next.push(other);
                }
            }
        }
        front = next;
    }
    depth
}
