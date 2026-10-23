//! Colour combinations (V大's week 2, 临摹色彩组合): a small abstract picture of a few colour
//! areas, copied by eye beside it: its overall tone first, then the middle tones, then the
//! hierarchy, never picked from the original. Pictures are generated the same for the same seed:
//! bands, patches or blocks (`areas_field.rs`), hard or soft edged, in colours related to a key
//! (near hues, values and purities) with one contrasting, at least half of them low in
//! saturation, at least one weak contrast between neighbours (under three just-noticeable steps)
//! and at least one strong one: the crops of rust, bark and abstract paintings V大 chooses.
//!
//! The picture is also a map of its areas on a `GRID` of cells: which area each cell lies in,
//! which areas meet, and each area's core, the cells deep enough inside it that neither its soft
//! edges nor a copy's slightly misplaced ones reach them. The copy is read there
//! (`areas_read.rs`).

use crate::areas_field::Field;
use crate::colour::{Rgb8, delta_e, from_hsv, hsv};
use crate::random::Random;

/// The map's cells across the frame and down it.
pub const GRID: usize = 128;
/// A contrast between neighbours this small or smaller is weak: under three just-noticeable
/// steps (Oklab 0.02 each).
pub const WEAK: f64 = 0.06;
/// One this large or larger is strong: more than seven such steps, about a step and a half of
/// Munsell value.
pub const STRONG: f64 = 0.15;
/// Every pair of neighbours differs by at least this (Oklab), so every area shows.
const LEAST: f64 = 0.025;
/// A soft edge's half width, in frame units.
const SOFT: [f64; 2] = [0.03, 0.08];
/// The least share of the frame an area holds.
const LEAST_SHARE: f64 = 0.05;
/// Cells a core lies inside its area's edges, beyond any soft edge's reach: about 19 canvas
/// pixels on a 400-pixel frame, more than a copy's edge is commonly off.
const CORE: usize = 6;
/// The least cells a core holds.
const LEAST_CORE: usize = 24;
/// Cell edges two areas share to count as neighbours.
const SHARED: usize = 4;
/// Pictures and palettes laid before the generator keeps the last one whatever it is.
const TRIES: usize = 256;

/// How a colour combination's areas lie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arrangement {
    /// Bands across the frame.
    Strata,
    /// Patches round a few centres.
    Patches,
    /// Blocks splitting the frame.
    Blocks,
}

impl Arrangement {
    pub const ALL: [Self; 3] = [Self::Strata, Self::Patches, Self::Blocks];
}

/// A colour combination's picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Areas {
    pub arrangement: Arrangement,
    /// Each area's colour.
    pub colours: Vec<Rgb8>,
    /// Each area's edge's half width in frame units: 0 for a hard edge.
    pub soft: Vec<f64>,
    /// The areas that meet, each pair once, the lower index first.
    pub neighbours: Vec<(usize, usize)>,
    field: Field,
    /// The area of each cell, row by row.
    map: Vec<u8>,
    /// How deep each cell lies in its area, in cells (1 at an edge), capped at 255.
    depth: Vec<u8>,
}

impl Areas {
    /// The picture `arrangement` and `seed` make: shapes laid and coloured from the seed's
    /// sequence until they keep every rule (a few tries; after `TRIES` the last, whatever it is).
    #[must_use]
    pub fn generate(arrangement: Arrangement, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut areas = Self::laid(arrangement, &mut random);
        for _ in 1..TRIES {
            if areas.placed() {
                for _ in 0..16 {
                    if let Some(colours) = palette(&mut random, areas.count(), &areas.neighbours) {
                        areas.colours = colours;
                        return areas;
                    }
                }
            }
            areas = Self::laid(arrangement, &mut random);
        }
        areas
    }

    /// Shapes and edges from `random`, every area grey.
    fn laid(arrangement: Arrangement, random: &mut Random) -> Self {
        let index = Arrangement::ALL.iter().position(|a| *a == arrangement);
        let field = Field::generate(index.unwrap_or(0), random);
        let count = field.count();
        let map = map(&field);
        let depth = depth(&map);
        Self {
            arrangement,
            colours: vec![[128; 3]; count],
            soft: softness(random, (&map, &depth), count),
            neighbours: neighbours(&map, count),
            field,
            map,
            depth,
        }
    }

    /// Whether the shapes keep their rules: four areas or more, each at least `LEAST_SHARE` of
    /// the frame with a core of `LEAST_CORE` cells.
    fn placed(&self) -> bool {
        let held = |area: usize| self.map.iter().filter(|a| usize::from(**a) == area).count();
        self.count() >= 4
            && (0..self.count())
                .all(|area| held(area) >= least_cells() && self.core(area).len() >= LEAST_CORE)
    }

    /// How many areas there are.
    #[must_use]
    pub fn count(&self) -> usize {
        self.colours.len()
    }

    /// The area the cell `[column, row]` lies in.
    pub(crate) fn cell(&self, [column, row]: [usize; 2]) -> usize {
        usize::from(self.map[row.min(GRID - 1) * GRID + column.min(GRID - 1)])
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

    /// The picture, `side` pixels square, as 8-bit sRGB with alpha (opaque): each area in its
    /// colour, an edge soft over its half width or hard (blended over a pixel and a half).
    #[must_use]
    pub fn picture(&self, side: u32) -> Vec<u8> {
        let side = side.max(1);
        let mut pixels = Vec::with_capacity(side as usize * side as usize * 4);
        for y in 0..side {
            for x in 0..side {
                let p = [x, y].map(|v| (f64::from(v) + 0.5) / f64::from(side));
                let [r, g, b] = self.colour_at(p, 0.75 / f64::from(side));
                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }
        pixels
    }

    /// The colour the picture shows at the frame point `p`, a hard edge blended over `hard`
    /// (frame units) either side.
    #[must_use]
    pub fn colour_at(&self, p: [f64; 2], hard: f64) -> Rgb8 {
        let at = self.field.at(p);
        let (Some(own), Some(other)) = (self.colours.get(at.area), self.colours.get(at.across))
        else {
            return [128; 3];
        };
        let width = self.soft[at.area].max(self.soft[at.across]).max(hard);
        let t = (at.edge / width).clamp(0.0, 1.0);
        // Half each at the edge, all its own `width` inside.
        let own_share = 0.5 + 0.5 * t * t * (3.0 - 2.0 * t);
        let mixed =
            |k: usize| f64::from(other[k]) + (f64::from(own[k]) - f64::from(other[k])) * own_share;
        [0, 1, 2].map(|k| mixed(k).round().clamp(0.0, 255.0) as u8)
    }
}

/// The least cells an area holds.
fn least_cells() -> usize {
    (LEAST_SHARE * (GRID * GRID) as f64) as usize
}

/// The area of each cell's middle, row by row.
fn map(field: &Field) -> Vec<u8> {
    let mut map = Vec::with_capacity(GRID * GRID);
    for row in 0..GRID {
        for column in 0..GRID {
            let p = [column, row].map(|v| (v as f64 + 0.5) / GRID as f64);
            map.push(u8::try_from(field.at(p).area).unwrap_or(u8::MAX));
        }
    }
    map
}

/// The pairs of areas sharing at least `SHARED` cell edges.
fn neighbours(map: &[u8], count: usize) -> Vec<(usize, usize)> {
    let mut shared = vec![0usize; count * count];
    for row in 0..GRID {
        for column in 0..GRID {
            let here = usize::from(map[row * GRID + column]);
            let right = (column + 1 < GRID).then(|| map[row * GRID + column + 1]);
            let below = (row + 1 < GRID).then(|| map[(row + 1) * GRID + column]);
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

/// How deep each cell lies in its area: 1 beside another area's cell (the frame's edge is no
/// edge), one more a step (of eight neighbours) further in, capped at 255.
fn depth(map: &[u8]) -> Vec<u8> {
    let mut depth = vec![u8::MAX; map.len()];
    let mut front = Vec::new();
    let around = |cell: usize| {
        let (row, column) = ((cell / GRID) as isize, (cell % GRID) as isize);
        (-1..=1)
            .flat_map(move |dy| (-1..=1).map(move |dx| (row + dy, column + dx)))
            .filter(|(r, c)| (0..GRID as isize).contains(r) && (0..GRID as isize).contains(c))
            .map(|(r, c)| r as usize * GRID + c as usize)
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

/// Each area's edge: about half of them soft, only where the area is deep enough to keep a
/// core inside the blend.
fn softness(random: &mut Random, (map, depth): (&[u8], &[u8]), count: usize) -> Vec<f64> {
    (0..count)
        .map(|area| {
            let deepest = (0..map.len())
                .filter(|cell| usize::from(map[*cell]) == area)
                .map(|cell| usize::from(depth[cell]))
                .max()
                .unwrap_or(0);
            let width = random.range(SOFT[0], SOFT[1]);
            let room = deepest as f64 >= (CORE + 6) as f64 + 2.0 * width * GRID as f64;
            if random.unit() < 0.5 && room {
                width
            } else {
                0.0
            }
        })
        .collect()
}

/// Colours for `count` areas meeting as `neighbours`: related to a key with one area
/// contrasting and one pair of neighbours a weak contrast; `None` when they miss a rule (every
/// neighbour pair at least `LEAST` apart, a weak and a strong contrast among them, half the
/// areas or more low in saturation).
fn palette(random: &mut Random, count: usize, neighbours: &[(usize, usize)]) -> Option<Vec<Rgb8>> {
    let pick =
        |random: &mut Random, len: usize| ((random.unit() * len as f64) as usize).min(len - 1);
    let (weak_a, weak_b) = *neighbours.get(pick(random, neighbours.len().max(1)))?;
    let others: Vec<usize> = (0..count)
        .filter(|a| ![weak_a, weak_b].contains(a))
        .collect();
    let contrast = *others.get(pick(random, others.len().max(1)))?;
    let key = [
        random.range(0.0, 360.0),
        random.range(0.08, 0.45),
        random.range(0.3, 0.75),
    ];
    let mut colours: Vec<[f64; 3]> = (0..count)
        .map(|area| {
            if area == contrast {
                let away = random.range(0.15, 0.35) * random.sign();
                let value = (key[2] + away).clamp(0.14, 0.93);
                [
                    key[0] + 180.0 + random.range(-35.0, 35.0),
                    random.range(0.2, 0.6),
                    value,
                ]
            } else {
                [
                    key[0] + random.range(-30.0, 30.0),
                    (key[1] + random.range(-0.12, 0.12)).clamp(0.03, 0.7),
                    (key[2] + random.range(-0.25, 0.25)).clamp(0.14, 0.93),
                ]
            }
        })
        .collect();
    let [h, s, v] = colours[weak_a];
    colours[weak_b] = [
        h + random.range(4.0, 12.0) * random.sign(),
        (s + random.range(0.01, 0.05) * random.sign()).clamp(0.02, 0.7),
        (v + random.range(0.02, 0.05) * random.sign()).clamp(0.12, 0.95),
    ];
    let colours: Vec<Rgb8> = colours.into_iter().map(from_hsv).collect();
    let contrasts: Vec<f64> = (neighbours.iter())
        .map(|(a, b)| delta_e(colours[*a], colours[*b]))
        .collect();
    let low = colours.iter().filter(|c| hsv(**c)[1] < 0.3).count();
    let kept = contrasts.iter().all(|c| *c >= LEAST)
        && contrasts.iter().any(|c| *c <= WEAK)
        && contrasts.iter().any(|c| *c >= STRONG)
        && 2 * low >= count;
    kept.then_some(colours)
}
