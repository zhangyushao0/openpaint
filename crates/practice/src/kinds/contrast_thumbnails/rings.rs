//! Where a contrast thumbnail's focus is printed (S154, V大's 先定重点): rings on a composition's
//! big shapes, each round a place where two of its masses meet, so a contrast can be made there
//! with the shapes given. The composition's own focus is where its masses' greys step the most
//! (`Steps` on its answer, the one nearest the frame's middle among equals); the first level rings
//! it, the second rings a place `MOVED_BY` from it, so the thumbnail must move the picture's emphasis
//! (Proko's variants of one reference); the third adds a second ring `MOVED_BY` from the first, to be
//! made clearly weaker than it and stronger than the rest (several accents of different strength).
//! The same for the same composition and seed: printed once with the sheet, found again at Compare.

use super::steps::Steps;
use crate::compositions::value_masses::ValueMasses;
use crate::maths::path::{Point, distance};
use crate::maths::random::{Random, derived};

/// A ring's radius, frame units (a share of the frame's longer side): room for a contrast and a
/// little round it, small enough to be a place.
pub const RING: f64 = 0.1;
/// How far a moved ring lies from the composition's own focus, and two rings from each other,
/// frame units.
pub const MOVED_BY: f64 = 0.3;
/// The least step a ring's middle stands on, Oklab lightness: about two masses meeting there.
const EDGE: f64 = 0.15;
/// Candidates every this many cells across and down.
const EVERY: usize = 3;
/// How far inside the frame a ring keeps, past its radius, frame units.
const INSIDE: f64 = 0.02;
/// The seed's own sequence for the rings.
const RINGS_SEED: u64 = 0x5154_0031;

/// How a thumbnail's emphasis is laid: in the course's order of the levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Emphasis {
    /// One ring, where the composition's own focus is.
    Own,
    /// One ring away from it.
    Moved,
    /// Two rings, the second (green) to be weaker than the first and stronger than the rest.
    Two,
}

impl Emphasis {
    pub const ALL: [Self; 3] = [Self::Own, Self::Moved, Self::Two];
}

/// A ring printed on the thumbnail: its middle and radius (frame units) and whether it is the
/// second accent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    pub centre: Point,
    pub radius: f64,
    pub second: bool,
}

impl Ring {
    /// Whether the frame point `p` lies in it.
    #[must_use]
    pub fn holds(&self, p: Point) -> bool {
        distance(self.centre, p) <= self.radius
    }

    /// Its line, a closed polygon of 48 sides (frame units).
    #[must_use]
    pub fn line(&self) -> Vec<Point> {
        (0..=48)
            .map(|k| {
                let turn = std::f64::consts::TAU * f64::from(k) / 48.0;
                let [x, y] = self.centre;
                [x + self.radius * turn.cos(), y + self.radius * turn.sin()]
            })
            .collect()
    }
}

/// The rings `emphasis` lays on the composition whose answer is `masses`, filling a frame of
/// `extent` (frame units), seeded `seed`.
#[must_use]
pub fn rings(masses: &ValueMasses, extent: Point, emphasis: Emphasis, seed: u64) -> Vec<Ring> {
    let candidates = candidates(masses, extent);
    let middle = [extent[0] / 2.0, extent[1] / 2.0];
    let focus = (candidates.iter().copied())
        .max_by(|(a, p), (b, q)| {
            let nearer = distance(*q, middle).total_cmp(&distance(*p, middle));
            a.total_cmp(b).then(nearer)
        })
        .map_or(middle, |(_, p)| p);
    let edged: Vec<Point> = (candidates.iter())
        .filter(|(step, _)| *step >= EDGE)
        .map(|(_, p)| *p)
        .collect();
    let mut random = Random::new(derived(seed, RINGS_SEED));
    let ring = |centre: Point, second: bool| Ring {
        centre,
        radius: RING,
        second,
    };
    match emphasis {
        Emphasis::Own => vec![ring(focus, false)],
        Emphasis::Moved => vec![ring(away(&edged, focus, &mut random), false)],
        Emphasis::Two => {
            let first = away(&edged, focus, &mut random);
            vec![
                ring(first, false),
                ring(away(&edged, first, &mut random), true),
            ]
        }
    }
}

/// Each candidate middle (frame units, inside the frame by a ring and a little) with the step the
/// composition's greys make there.
fn candidates(masses: &ValueMasses, extent: Point) -> Vec<(f64, Point)> {
    let steps = Steps::of(masses.size, &masses.picture());
    let [w, h] = steps.size.map(|side| side as usize);
    let keep = RING + INSIDE;
    (0..h)
        .step_by(EVERY)
        .flat_map(|y| (0..w).step_by(EVERY).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = [
                (x as f64 + 0.5) / w as f64 * extent[0],
                (y as f64 + 0.5) / h as f64 * extent[1],
            ];
            (steps.steps[y * w + x], p)
        })
        .filter(|(_, [x, y])| {
            (keep..=extent[0] - keep).contains(x) && (keep..=extent[1] - keep).contains(y)
        })
        .collect()
}

/// A place of `edged` at least `MOVED_BY` from `from`, picked by `random`; the farthest where none
/// lies so far, `from` itself where there are none.
fn away(edged: &[Point], from: Point, random: &mut Random) -> Point {
    let far: Vec<Point> = (edged.iter().copied())
        .filter(|p| distance(*p, from) >= MOVED_BY)
        .collect();
    if far.is_empty() {
        return (edged.iter().copied())
            .max_by(|p, q| distance(*p, from).total_cmp(&distance(*q, from)))
            .unwrap_or(from);
    }
    let at = ((random.unit() * far.len() as f64) as usize).min(far.len() - 1);
    far[at]
}
