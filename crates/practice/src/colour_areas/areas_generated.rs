//! S82's generated colour combinations (1.5.0 to 1.18.1): bands, patches or blocks
//! (`areas_field.rs`), hard or soft edged, in colours related to a key (near hues, values and
//! purities) with one contrasting, at least half of them low in saturation, at least one weak
//! contrast between neighbours (under three just-noticeable steps) and at least one strong one,
//! the same for the same seed. Retired from the exercises by S101 (the author: they vary too
//! little beside real crops); they stay as the records made of them show them, as the How's
//! example and as pictures of known areas the found answer is validated on
//! (`areas_found.rs`).

use crate::colour_areas::areas::{Areas, CORE, GRID, LEAST_CORE, LEAST_SHARE, STRONG, WEAK};
use crate::colour_areas::areas_field::Field;
use crate::maths::colour::{Rgb8, delta_e, from_hsv, hsv};
use crate::maths::random::Random;

/// Every pair of neighbours differs by at least this (Oklab), so every area shows.
const LEAST: f64 = 0.025;
/// A soft edge's half width, in frame units.
const SOFT: [f64; 2] = [0.03, 0.08];
/// Pictures and palettes laid before the generator keeps the last one whatever it is.
const TRIES: usize = 256;

/// How a generated combination's areas lie.
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

/// A generated colour combination: its square picture's field and its answer.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedAreas {
    pub arrangement: Arrangement,
    pub areas: Areas,
    field: Field,
}

impl GeneratedAreas {
    /// The picture `arrangement` and `seed` make: shapes laid and coloured from the seed's
    /// sequence until they keep every rule (a few tries; after `TRIES` the last, whatever it is).
    #[must_use]
    pub fn generate(arrangement: Arrangement, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut generated = Self::laid(arrangement, &mut random);
        for _ in 1..TRIES {
            if generated.placed() {
                for _ in 0..16 {
                    let areas = &generated.areas;
                    if let Some(colours) = palette(&mut random, areas.count(), &areas.neighbours) {
                        generated.areas.colours = colours;
                        return generated;
                    }
                }
            }
            generated = Self::laid(arrangement, &mut random);
        }
        generated
    }

    /// Shapes and edges from `random`, every area grey.
    fn laid(arrangement: Arrangement, random: &mut Random) -> Self {
        let index = Arrangement::ALL.iter().position(|a| *a == arrangement);
        let field = Field::generate(index.unwrap_or(0), random);
        let count = field.count();
        let map = map(&field);
        let hard = Areas::laid([GRID; 2], map, vec![[128; 3]; count], vec![0.0; count]);
        let soft = softness(random, &hard, count);
        let areas = Areas::laid([GRID; 2], hard.map().to_vec(), hard.colours, soft);
        Self {
            arrangement,
            areas,
            field,
        }
    }

    /// Whether the shapes keep their rules: four areas or more, each at least `LEAST_SHARE` of
    /// the frame with a core of `LEAST_CORE` cells.
    fn placed(&self) -> bool {
        let areas = &self.areas;
        let sizes = areas.sizes();
        let least = (LEAST_SHARE * (GRID * GRID) as f64) as usize;
        areas.count() >= 4
            && (0..areas.count())
                .all(|area| sizes[area] >= least && areas.core(area).len() >= LEAST_CORE)
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
        let (colours, soft) = (&self.areas.colours, &self.areas.soft);
        let (Some(own), Some(other)) = (colours.get(at.area), colours.get(at.across)) else {
            return [128; 3];
        };
        let width = soft[at.area].max(soft[at.across]).max(hard);
        let t = (at.edge / width).clamp(0.0, 1.0);
        // Half each at the edge, all its own `width` inside.
        let own_share = 0.5 + 0.5 * t * t * (3.0 - 2.0 * t);
        let mixed =
            |k: usize| f64::from(other[k]) + (f64::from(own[k]) - f64::from(other[k])) * own_share;
        [0, 1, 2].map(|k| mixed(k).round().clamp(0.0, 255.0) as u8)
    }
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

/// Each area's edge: about half of them soft, only where the area is deep enough to keep a
/// core inside the blend.
fn softness(random: &mut Random, hard: &Areas, count: usize) -> Vec<f64> {
    (0..count)
        .map(|area| {
            let deepest = hard.deepest(area);
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
