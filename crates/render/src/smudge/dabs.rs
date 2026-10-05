//! A smudge's contacts (draweave's `DabWalker`, at one node per canvas pixel): walking the
//! stroke's curve (its spans flattened at canvas resolution, S34), a contact every `clamp(r / 4,
//! 0.25, 1)` pixels along it, `r` the radius where the piece being walked ends. Each moves the
//! colour by its step `d` and weighs it by the tip's profile times strength. The samples already
//! follow the StreamLine (the app's stroke intent, with no catch-up for a smudge), so the contacts
//! trail the path strictly: walking the spans at once or a few at a time gives the same contacts,
//! bit for bit.

use std::collections::BTreeSet;
use std::ops::Range;

use openpaint_core::{Deposit, StrokeSample, StrokeStyle, Tip};

use super::RESIDUAL_TILE;
use crate::geom::Bounds;
use crate::stroke::{Path, spans};

/// Smallest and largest contact radius in pixels: the stamp's scratch covers one contact.
pub(crate) const MIN_RADIUS: f32 = 0.5;
pub(crate) const MAX_RADIUS: f32 = 300.0;
/// Contact spacing in pixels, a quarter of the radius between these.
const MIN_SPACING: f32 = 0.25;
const MAX_SPACING: f32 = 1.0;
/// A contact's pull reaches this far past its radius: its step (at most a pixel) and the
/// bilinear neighbours it reads.
const READ_MARGIN: u32 = 3;
/// How far a smudge's segments reach past the radius: a contact lies within a spacing (at most
/// a pixel) of the path, and its nodes within half a pixel more of its circle.
const REACH: f32 = 3.0;

/// The weight's cross-section over the squared distance `q` from the centre in radii.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kernel {
    /// `1 - smoothstep(0.64, 1, q)`: a hard tip.
    Hard = 0,
    /// `(1 - q)^2`.
    Dome = 1,
    /// `(1 - q)^3`.
    Bell = 2,
    /// `1 - q^4`.
    Flat = 3,
}

/// The weight profile of a recipe's tip.
pub(crate) fn kernel(style: &StrokeStyle) -> Kernel {
    let recipe = style.recipe();
    match recipe.deposit() {
        Deposit::Cover if recipe.softness() > 0.0 => Kernel::Dome,
        Deposit::Cover => Kernel::Hard,
        Deposit::Accumulate(accumulation) => match accumulation.tip() {
            Tip::Flat => Kernel::Flat,
            Tip::Bell => Kernel::Bell,
        },
    }
}

/// One contact, in canvas pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Dab {
    pub(crate) centre: [f32; 2],
    /// From the contact before to this one.
    pub(crate) step: [f32; 2],
    pub(crate) radius: f32,
    pub(crate) strength: f32,
    pub(crate) kernel: Kernel,
}

impl Dab {
    /// The nodes it can change, `[x, y, width, height]`, inside a canvas of `size` pixels:
    /// centres closer than the radius.
    pub(crate) fn rect(&self, size: [u32; 2]) -> Option<[u32; 4]> {
        self.nodes(size, 0)
    }

    /// The nodes it reads: its own and those its step and their neighbours reach.
    pub(crate) fn read_rect(&self, size: [u32; 2]) -> Option<[u32; 4]> {
        self.nodes(size, READ_MARGIN)
    }

    fn nodes(&self, size: [u32; 2], margin: u32) -> Option<[u32; 4]> {
        let span = |centre: f32, side: u32| {
            let low = (centre - self.radius - 0.5).floor() as i64 - i64::from(margin);
            let high = (centre + self.radius - 0.5).ceil() as i64 + i64::from(margin);
            let (low, high) = (low.max(0), high.min(i64::from(side) - 1));
            (low <= high).then_some((low as u32, (high - low + 1) as u32))
        };
        let (x, width) = span(self.centre[0], size[0])?;
        let (y, height) = span(self.centre[1], size[1])?;
        Some([x, y, width, height])
    }
}

/// Walks a smudge's curve (S34: its spans flattened at canvas resolution, `Segment::pieces`)
/// into contacts, remembering where it got to.
#[derive(Debug, Clone, Default)]
pub(crate) struct DabWalker {
    last: Option<[f32; 2]>,
    /// Spans walked.
    walked: usize,
}

impl DabWalker {
    /// Appends the contacts of the spans after those walked before: of every span when the
    /// stroke is `finished`, else only of those no later sample changes (`Path::settled`), so
    /// an in-progress smudge trails its last two spans and catches up when it is committed.
    pub(crate) fn advance(
        &mut self,
        style: &StrokeStyle,
        (samples, finished): (&[StrokeSample], bool),
        out: &mut Vec<Dab>,
    ) {
        let kernel = kernel(style);
        let path = Path::of(samples);
        let end = if finished {
            path.spans()
        } else {
            path.settled()
        };
        for segment in spans(style, samples, &path, self.walked.min(end)..end) {
            for piece in segment.pieces(1.0) {
                let mut last = *self.last.get_or_insert(piece.a);
                let point = piece.b;
                let radius = piece.radii[1].clamp(MIN_RADIUS, MAX_RADIUS);
                let spacing = (radius * 0.25).clamp(MIN_SPACING, MAX_SPACING);
                let strength = (style.opacity() * piece.pressures[1]).clamp(0.0, 1.0);
                loop {
                    let delta = [point[0] - last[0], point[1] - last[1]];
                    let distance = delta[0].hypot(delta[1]);
                    if distance < spacing {
                        break;
                    }
                    let next = [
                        last[0] + delta[0] / distance * spacing,
                        last[1] + delta[1] / distance * spacing,
                    ];
                    out.push(Dab {
                        centre: next,
                        step: [next[0] - last[0], next[1] - last[1]],
                        radius,
                        strength,
                        kernel,
                    });
                    last = next;
                }
                self.last = Some(last);
            }
        }
        self.walked = self.walked.max(end);
    }
}

/// Every contact of a whole stroke.
pub(crate) fn dabs(style: &StrokeStyle, samples: &[StrokeSample]) -> Vec<Dab> {
    let mut out = Vec::new();
    DabWalker::default().advance(style, (samples, true), &mut out);
    out
}

/// The residual tiles the contacts read, as row-major indices in a canvas of `size` pixels.
pub(crate) fn read_tiles(dabs: &[Dab], size: [u32; 2]) -> BTreeSet<u32> {
    let columns = size[0].div_ceil(RESIDUAL_TILE);
    let mut tiles = BTreeSet::new();
    for rect in dabs.iter().filter_map(|dab| dab.read_rect(size)) {
        let [x, y, width, height] = rect;
        for row in y / RESIDUAL_TILE..=(y + height - 1) / RESIDUAL_TILE {
            for column in x / RESIDUAL_TILE..=(x + width - 1) / RESIDUAL_TILE {
                tiles.insert(row * columns + column);
            }
        }
    }
    tiles
}

/// Where a smudge's spans `range` (of `path`) can change pixels: per span, the box of its
/// curve's points widened by the larger radius and `REACH`. A contact reached while walking a
/// piece lies within a spacing of it and has a radius between the span's ends.
pub(crate) fn smudge_reach<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
    path: &'a Path,
    range: Range<usize>,
) -> impl Iterator<Item = Bounds> + 'a {
    spans(style, samples, path, range).map(|segment| {
        let [ra, rb] = segment.radii().map(|r| r.clamp(MIN_RADIUS, MAX_RADIUS));
        segment.hull(ra.max(rb) + REACH)
    })
}

/// `smudge_reach` of every span of a finished smudge.
pub(crate) fn whole_reach(style: &StrokeStyle, samples: &[StrokeSample]) -> Vec<Bounds> {
    let path = Path::of(samples);
    smudge_reach(style, samples, &path, 0..path.spans()).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use openpaint_core::{BrushRecipe, CanvasPoint, Color, StrokeTool};

    fn style() -> StrokeStyle {
        let recipe = Arc::new(BrushRecipe::plain());
        let style = StrokeStyle::new(recipe, 24.0, 0.7, Color::BLACK).expect("style");
        style.with_tool(StrokeTool::Smudge)
    }

    fn path() -> Vec<StrokeSample> {
        (0..40u16)
            .map(|i| {
                let t = f32::from(i);
                let position = CanvasPoint::new(20.0 + t * 3.7, 30.0 + (t * 0.3).sin() * 12.0);
                let pressure = 0.4 + 0.015 * t;
                StrokeSample::new(
                    position.expect("point"),
                    pressure,
                    0.0,
                    0.0,
                    u32::from(i) * 8,
                )
                .expect("sample")
            })
            .collect()
    }

    #[test]
    fn contacts_are_the_same_walked_at_once_or_sample_by_sample() {
        let (style, samples) = (style(), path());
        let whole = dabs(&style, &samples);
        let mut walker = DabWalker::default();
        let mut stepwise = Vec::new();
        for end in 1..=samples.len() {
            walker.advance(
                &style,
                (&samples[..end], end == samples.len()),
                &mut stepwise,
            );
        }
        assert_eq!(whole, stepwise);
        assert!(whole.len() > 100, "{} contacts", whole.len());
        assert!(
            whole
                .iter()
                .all(|dab| dab.step[0].hypot(dab.step[1]) <= 1.0 + 1e-5)
        );
    }

    #[test]
    fn the_reach_holds_every_contact_and_the_tiles_every_read() {
        let (style, samples) = (style(), path());
        let reach = whole_reach(&style, &samples);
        let size = [200, 100];
        for dab in dabs(&style, &samples) {
            let [x, y, width, height] = dab.rect(size).expect("inside");
            let inside = reach.iter().any(|Bounds([x0, y0, x1, y1])| {
                *x0 <= x as f32
                    && *y0 <= y as f32
                    && *x1 >= (x + width) as f32 - 1.0
                    && *y1 >= (y + height) as f32 - 1.0
            });
            assert!(inside, "{dab:?}");
        }
        let tiles = read_tiles(&dabs(&style, &samples), size);
        assert!(tiles.contains(&0) && tiles.len() < 21, "{tiles:?}");
    }
}
