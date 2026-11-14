//! A picture's silhouette outline (S80): what Compare lays over a copy of a single shape. It is
//! derived once, when a pack is built (and the official pack's are looked at on its contact
//! sheets) or when one of the user's own pictures is imported, and kept with the picture. The
//! derivation here is the device's, for the user's own pictures; the pack builder outlines a
//! picture on a clear ground the same way and any other from a segmentation's matte
//! (`outline_matte.rs`, S81).
//!
//! The way: the picture reduced to a working grid of `WORK_SIDE` (`outline_ground.rs`); its
//! ground read from its edge, clear (transparent) or a plain colour changing smoothly (each side's
//! Oklab profile, the object's own edge pixels left out, laid across as a Coons patch); the object
//! the pixels that differ from the ground, in parts that each differ strongly somewhere; the ground
//! flooded in from the edge, the rest the silhouette, enclosed pockets of clear ground kept as holes
//! (`outline_mask.rs`); its contours traced and thinned (`outline_trace.rs`).
//!
//! An outline that is wrong must never be shown, so the derivation refuses rather than guesses:
//! an edge that is no plain ground (a busy photo), nothing standing out, several parts where one
//! object should be, an object cut by the picture's edge, a silhouette too small or too large, a
//! contour too ragged for an object's. A refused picture has no outline: Compare shows the
//! picture itself, faint, in place.

use std::fmt;

use crate::maths::path::Point;
use crate::outlines::outline_ground::{Grid, Ground, difference, grid};
use crate::outlines::outline_mask::silhouette;
use crate::outlines::outline_trace::{area, contours, perimeter, thinned};

/// How far a thinned contour may stray from the traced one, in grid pixels.
const THINNING: f64 = 0.35;
/// On a plain ground, the share of the picture's edge the silhouette may touch.
const MOST_CUT: f64 = 0.06;
/// The silhouette's share of the picture, and its box's least share of either side.
const LEAST_AREA: f64 = 0.015;
const MOST_AREA: f64 = 0.92;
const LEAST_SPAN: f64 = 0.12;
/// A contour enclosing less than this share of the silhouette is left out.
const LEAST_CONTOUR: f64 = 0.002;
/// The roundness (4 pi area / perimeter squared, 1 for a circle) under which a silhouette is
/// too ragged to be an object's, and the most contours and points an outline holds.
const LEAST_ROUNDNESS: f64 = 0.012;
const MOST_CONTOURS: usize = 24;
const MOST_POINTS: usize = 6000;

/// A picture's silhouette: closed contours in the picture's pixels (each without its first point
/// repeated), the largest first; the outer edges of its parts and the holes it encloses.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub contours: Vec<Vec<Point>>,
}

/// Why a picture has no outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineRefusal {
    /// The picture's edge is neither transparent nor one plain ground.
    Ground,
    /// Nothing stands out from the ground.
    Nothing,
    /// Several parts, none of them the object.
    Parts,
    /// The object runs off the picture's edge.
    Cut,
    /// The silhouette is too small or too large in the picture.
    Size,
    /// The contour is too ragged, or too long, for an object's.
    Ragged,
    /// The pixels do not make a picture of the size given.
    Pixels,
}

impl fmt::Display for OutlineRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ground => "its edge is no plain ground",
            Self::Nothing => "nothing stands out from the ground",
            Self::Parts => "several parts, none of them the object",
            Self::Cut => "the object runs off the picture's edge",
            Self::Size => "the silhouette is too small or too large",
            Self::Ragged => "the contour is too ragged for an object's",
            Self::Pixels => "the pixels are not a picture of its size",
        })
    }
}

impl std::error::Error for OutlineRefusal {}

impl Outline {
    /// Its contours as closed lines (the first point repeated at the end), to draw or fit.
    #[must_use]
    pub fn lines(&self) -> Vec<Vec<Point>> {
        self.contours
            .iter()
            .map(|contour| {
                let mut line = contour.clone();
                line.extend(contour.first().copied());
                line
            })
            .collect()
    }
}

/// The silhouette outline of the picture `rgba` (8-bit sRGB, straight alpha, `size` pixels).
///
/// # Errors
/// Why it has none (`OutlineRefusal`).
pub fn outline(size: [u32; 2], rgba: &[u8]) -> Result<Outline, OutlineRefusal> {
    let grid = grid(size, rgba).ok_or(OutlineRefusal::Pixels)?;
    let cells = [grid.width, grid.height];
    let difference = difference(&grid)?;
    let inside = silhouette(cells, &difference)?;
    let cut = matches!(difference.ground, Ground::Plain { .. });
    check_silhouette(cells, &inside, cut)?;
    traced(size, &grid, &inside)
}

/// The outline of the silhouette `inside` on `grid` (of a picture of `size` pixels): its
/// contours traced, thinned and checked, in the picture's pixels.
pub(crate) fn traced(
    size: [u32; 2],
    grid: &Grid,
    inside: &[bool],
) -> Result<Outline, OutlineRefusal> {
    let cells = [grid.width, grid.height];
    let filled = inside.iter().filter(|i| **i).count() as f64;
    let mut traced: Vec<Vec<Point>> = contours(cells, inside)
        .into_iter()
        .filter(|contour| area(contour).abs() >= LEAST_CONTOUR * filled)
        .map(|contour| thinned(&contour, THINNING))
        .collect();
    traced.sort_by(|a, b| area(b).abs().total_cmp(&area(a).abs()));
    check_contours(&traced)?;
    // In the picture's pixels, at the f32 precision a pack keeps.
    let [across, down] = grid.scale;
    let [width, height] = size.map(f64::from);
    let kept = |v: f64, side: f64| f64::from(v.clamp(0.0, side) as f32).min(side);
    let contours = traced
        .into_iter()
        .map(|contour| {
            (contour.iter())
                .map(|p| [kept(p[0] * across, width), kept(p[1] * down, height)])
                .collect()
        })
        .collect();
    Ok(Outline { contours })
}

/// The silhouette's size and, where it stands on a ground (`cut`), how much of the picture's
/// edge it touches.
pub(crate) fn check_silhouette(
    [width, height]: [usize; 2],
    inside: &[bool],
    cut: bool,
) -> Result<(), OutlineRefusal> {
    let filled = inside.iter().filter(|i| **i).count();
    let share = filled as f64 / inside.len().max(1) as f64;
    let (mut left, mut right, mut top, mut bottom) = (width, 0, height, 0);
    let mut edge = 0;
    for (i, _) in inside.iter().enumerate().filter(|(_, i)| **i) {
        let (x, y) = (i % width, i / width);
        (left, right, top, bottom) = (left.min(x), right.max(x), top.min(y), bottom.max(y));
        if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
            edge += 1;
        }
    }
    let spans = [
        (right + 1).saturating_sub(left) as f64 / width as f64,
        (bottom + 1).saturating_sub(top) as f64 / height as f64,
    ];
    if !(LEAST_AREA..=MOST_AREA).contains(&share) || spans.iter().all(|s| *s < LEAST_SPAN) {
        return Err(OutlineRefusal::Size);
    }
    let ring = 2 * (width + height) - 4;
    if cut && edge as f64 / ring.max(1) as f64 > MOST_CUT {
        return Err(OutlineRefusal::Cut);
    }
    Ok(())
}

/// The contours an outline may hold: some, not too many or too long, the largest round enough.
fn check_contours(contours: &[Vec<Point>]) -> Result<(), OutlineRefusal> {
    let Some(largest) = contours.first() else {
        return Err(OutlineRefusal::Nothing);
    };
    let points: usize = contours.iter().map(Vec::len).sum();
    let roundness =
        4.0 * std::f64::consts::PI * area(largest).abs() / perimeter(largest).powi(2).max(1e-9);
    if contours.len() > MOST_CONTOURS || points > MOST_POINTS || roundness < LEAST_ROUNDNESS {
        return Err(OutlineRefusal::Ragged);
    }
    Ok(())
}
