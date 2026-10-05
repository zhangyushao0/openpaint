//! `ActiveTrack`: which in-progress stroke a canvas target's coverage texture holds, so each
//! evaluation adds only the spans of its curve that are new since the last one. The texture
//! keeps only settled spans (`Path::settled`); the last two, which later samples may still
//! change, are drawn every evaluation on a copy and never kept (`OP_TAIL`), so a stroke drawn
//! a few samples at a time ends exactly as the finished stroke evaluated at once.

use std::ops::Range;

use openpaint_core::{
    ActiveStroke, Artwork, LayerId, RegionMask, StrokeSample, StrokeStyle, StrokeTool,
};

use crate::geom::PixelRect;
use crate::stroke::{Path, span_regions, stroke_region};

/// The stroke whose coverage the texture holds, as far as it was evaluated, and the artwork
/// version it was last drawn over.
#[derive(Debug)]
struct Tracked {
    base: Artwork,
    layer: LayerId,
    /// On the layer's mask.
    on_mask: bool,
    style: StrokeStyle,
    clip: Option<RegionMask>,
    samples: Vec<StrokeSample>,
    /// The spans the texture holds.
    kept: usize,
    /// Every pixel the evaluated spans can have changed.
    region: Option<PixelRect>,
    /// What the unsettled spans reach: drawn last time (paint), or not drawn yet (a smudge's,
    /// whose contacts trail them until it is committed).
    tail: Option<PixelRect>,
}

/// How one evaluation draws the in-progress stroke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Step {
    /// Clear the coverage texture first: a different stroke starts.
    pub(crate) reset: bool,
    /// The spans to add to the coverage texture: settled since the last evaluation. The spans
    /// after them are the tail, drawn every time.
    pub(crate) kept: Range<usize>,
    /// The pixels to evaluate for the stroke: the new spans, the tail and the tail drawn last.
    pub(crate) fresh_region: Option<PixelRect>,
    /// Pixels to evaluate again from the scene when the tracked stroke is gone. Dropped (the
    /// artwork stayed the same version): everything it drew. Committed: what its spans gained
    /// or changed after its last evaluation. Anything else: nothing, the caller names its own
    /// changes.
    pub(crate) redraw: Option<PixelRect>,
}

#[derive(Debug, Default)]
pub(crate) struct ActiveTrack {
    tracked: Option<Tracked>,
}

fn union(a: Option<PixelRect>, b: Option<PixelRect>) -> Option<PixelRect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.union(&b)),
        (a, b) => a.or(b),
    }
}

impl ActiveTrack {
    /// Moves to `active` drawn over `artwork` and says what to evaluate for it. `drawn` is false
    /// when the stroke has no place in the scene (its layer is hidden); nothing is then tracked.
    pub(crate) fn advance(
        &mut self,
        artwork: &Artwork,
        active: Option<&ActiveStroke>,
        drawn: bool,
    ) -> Step {
        let active = active.filter(|_| drawn);
        let continuing = match (&self.tracked, active) {
            (Some(tracked), Some(active)) => tracked.continues_into(active),
            _ => false,
        };
        let redraw = match &self.tracked {
            Some(tracked) if !continuing => tracked.gone(artwork),
            _ => None,
        };
        let Some(active) = active else {
            self.tracked = None;
            return Step {
                reset: false,
                kept: 0..0,
                fresh_region: None,
                redraw,
            };
        };
        let (style, samples) = (active.style(), active.samples());
        let path = Path::of(samples);
        let (settled, spans) = (path.settled(), path.spans());
        let tracked = self.tracked.as_ref().filter(|_| continuing);
        let from = tracked.map_or(0, |tracked| tracked.kept);
        let grew = tracked.is_none_or(|tracked| tracked.samples.len() < samples.len());
        let smudge = style.tool() == StrokeTool::Smudge;
        let regions = |range: Range<usize>| {
            span_regions(style, samples, &path, range).reduce(|all, next| all.union(&next))
        };
        let tail = regions(settled..spans);
        let fresh_region = if grew {
            let new = regions(from..settled);
            let before = tracked.and_then(|tracked| tracked.tail);
            if smudge {
                new
            } else {
                union(union(new, tail), before)
            }
        } else {
            None
        };
        self.record(artwork, active, (settled, tail), fresh_region);
        Step {
            reset: !continuing,
            kept: from..settled,
            fresh_region,
            redraw,
        }
    }

    /// Forgets the stroke, so the next evaluation starts its coverage over.
    pub(crate) fn forget(&mut self) {
        self.tracked = None;
    }

    fn record(
        &mut self,
        artwork: &Artwork,
        active: &ActiveStroke,
        (kept, tail): (usize, Option<PixelRect>),
        region: Option<PixelRect>,
    ) {
        let continuing = self
            .tracked
            .as_ref()
            .is_some_and(|tracked| tracked.continues_into(active));
        let tracked = match self.tracked.as_mut() {
            Some(tracked) if continuing => tracked,
            _ => self.tracked.insert(Tracked {
                base: artwork.clone(),
                layer: active.layer(),
                on_mask: active.on_mask(),
                style: active.style().clone(),
                clip: active.clip().cloned(),
                samples: Vec::new(),
                kept: 0,
                region: None,
                tail: None,
            }),
        };
        tracked.base = artwork.clone();
        let done = tracked.samples.len();
        tracked.samples.extend_from_slice(&active.samples()[done..]);
        tracked.kept = kept;
        tracked.tail = tail;
        tracked.region = union(tracked.region, region);
    }
}

impl Tracked {
    /// True when `active` is this stroke with zero or more samples added.
    fn continues_into(&self, active: &ActiveStroke) -> bool {
        self.layer == active.layer()
            && self.on_mask == active.on_mask()
            && self.style == *active.style()
            && self.clip.as_ref() == active.clip()
            && active.samples().starts_with(&self.samples)
    }

    /// What to evaluate again now that the stroke is no longer drawn over `artwork`. Committed,
    /// it may have gained samples after its last evaluation, and its end may have moved as it
    /// lifted (StreamLine catching up with the pen): where it differs from what was drawn, both
    /// the old spans and the new are evaluated again; a smudge committed as drawn also shows its
    /// tail now. Its first sample never changes.
    fn gone(&self, artwork: &Artwork) -> Option<PixelRect> {
        if artwork.same_version(&self.base) {
            return self.region;
        }
        let layer = artwork.layer(self.layer)?;
        let strokes = match layer.mask().filter(|_| self.on_mask) {
            Some(mask) => mask.strokes(),
            None => layer.strokes(),
        };
        let stroke = strokes.last()?;
        let samples = stroke.samples();
        let shared = samples
            .iter()
            .zip(&self.samples)
            .take_while(|(now, drawn)| now == drawn)
            .count();
        let committed = *stroke.style() == self.style && shared > 0;
        if !committed {
            return None;
        }
        let smudge = self.style.tool() == StrokeTool::Smudge;
        if shared == samples.len() && shared == self.samples.len() {
            return self.tail.filter(|_| smudge);
        }
        let drawn = stroke_region(&self.style, &self.samples, shared);
        let now = stroke_region(&self.style, samples, shared);
        union(drawn, now)
    }
}

#[cfg(test)]
#[path = "active_tests.rs"]
mod tests;
