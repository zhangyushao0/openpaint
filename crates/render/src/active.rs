//! `ActiveTrack`: which in-progress stroke a canvas target's coverage texture holds, so each
//! evaluation adds only the segments that are new since the last one.

use std::ops::Range;

use openpaint_core::{ActiveStroke, Artwork, LayerId, StrokeSample, StrokeStyle};

use crate::geom::PixelRect;
use crate::scene::stroke_region;

/// The stroke whose coverage the texture holds, as far as it was evaluated, and the artwork
/// version it was last drawn over.
#[derive(Debug)]
struct Tracked {
    base: Artwork,
    layer: LayerId,
    style: StrokeStyle,
    samples: Vec<StrokeSample>,
    /// Every pixel the evaluated samples can have changed.
    region: Option<PixelRect>,
}

/// How one evaluation draws the in-progress stroke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Step {
    /// Clear the coverage texture first: a different stroke starts.
    pub(crate) reset: bool,
    /// The samples whose segments are fresh: the new ones and the one before them, all of them
    /// after a reset, or none.
    pub(crate) fresh: Range<usize>,
    /// The pixels the fresh segments can change.
    pub(crate) fresh_region: Option<PixelRect>,
    /// Pixels to evaluate again from the scene when the tracked stroke is gone. Dropped (the
    /// artwork stayed the same version): everything it drew. Committed: the segments it gained
    /// after its last evaluation. Anything else: nothing, the caller names its own changes.
    pub(crate) redraw: Option<PixelRect>,
}

#[derive(Debug, Default)]
pub(crate) struct ActiveTrack {
    tracked: Option<Tracked>,
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
                fresh: 0..0,
                fresh_region: None,
                redraw,
            };
        };
        let samples = active.samples();
        let (reset, fresh) = match &self.tracked {
            Some(tracked) if continuing => {
                let done = tracked.samples.len();
                let fresh = if samples.len() > done { done - 1 } else { done };
                (false, fresh..samples.len())
            }
            _ => (true, 0..samples.len()),
        };
        let fresh_region = stroke_region(active.style(), &samples[fresh.clone()]);
        self.record(artwork, active, reset, fresh_region);
        Step {
            reset,
            fresh,
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
        reset: bool,
        region: Option<PixelRect>,
    ) {
        let tracked = match self.tracked.as_mut() {
            Some(tracked) if !reset => tracked,
            _ => self.tracked.insert(Tracked {
                base: artwork.clone(),
                layer: active.layer(),
                style: *active.style(),
                samples: Vec::new(),
                region: None,
            }),
        };
        tracked.base = artwork.clone();
        let done = tracked.samples.len();
        tracked.samples.extend_from_slice(&active.samples()[done..]);
        tracked.region = match (tracked.region, region) {
            (Some(all), Some(new)) => Some(all.union(&new)),
            (all, new) => all.or(new),
        };
    }
}

impl Tracked {
    /// True when `active` is this stroke with zero or more samples added. A single sample is a
    /// dot that later segments replace rather than join, so a dot that grows starts over.
    fn continues_into(&self, active: &ActiveStroke) -> bool {
        let samples = active.samples();
        let grew_from_dot = self.samples.len() == 1 && samples.len() > 1;
        self.layer == active.layer()
            && self.style == *active.style()
            && samples.starts_with(&self.samples)
            && !grew_from_dot
    }

    /// What to evaluate again now that the stroke is no longer drawn over `artwork`.
    fn gone(&self, artwork: &Artwork) -> Option<PixelRect> {
        if artwork.same_version(&self.base) {
            return self.region;
        }
        let stroke = artwork.layer(self.layer)?.strokes().last()?;
        let samples = stroke.samples();
        let committed = *stroke.style() == self.style && samples.starts_with(&self.samples);
        if !committed || samples.len() == self.samples.len() {
            return None;
        }
        // From the last evaluated sample on; after a lone dot that is the whole stroke, whose
        // first segment covers the dot it replaces.
        let from = self.samples.len().saturating_sub(1);
        stroke_region(&self.style, &samples[from..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_core::{CanvasPoint, CanvasSize, Color, StrokeSession};

    fn sample(x: f32, time_ms: u32) -> StrokeSample {
        let position = CanvasPoint::new(x, 10.0).expect("point");
        StrokeSample::new(position, 1.0, 0.0, 0.0, time_ms).expect("sample")
    }

    fn begin(artwork: &Artwork, x: f32) -> StrokeSession {
        let style = StrokeStyle::new(4.0, 1.0, Color::BLACK).expect("style");
        artwork.begin_stroke(style, sample(x, 0)).expect("begin")
    }

    fn blank() -> Artwork {
        Artwork::new(CanvasSize::new(64, 64).expect("size"))
    }

    #[test]
    fn a_growing_stroke_adds_only_its_new_segments() {
        let base = blank();
        let mut track = ActiveTrack::default();
        let mut session = begin(&base, 1.0);
        let first = track.advance(&base, Some(session.active()), true);
        assert_eq!((first.reset, first.fresh), (true, 0..1));
        session.extend(&[sample(5.0, 4)]).expect("extend");
        let dot_grows = track.advance(&base, Some(session.active()), true);
        assert_eq!((dot_grows.reset, dot_grows.fresh), (true, 0..2));
        session
            .extend(&[sample(9.0, 8), sample(13.0, 12)])
            .expect("extend");
        let more = track.advance(&base, Some(session.active()), true);
        assert_eq!((more.reset, more.fresh, more.redraw), (false, 1..4, None));
        let same = track.advance(&base, Some(session.active()), true);
        assert_eq!(
            (same.reset, same.fresh, same.fresh_region),
            (false, 4..4, None)
        );
    }

    #[test]
    fn a_cancelled_stroke_is_erased_and_a_committed_one_is_not() {
        let base = blank();
        let mut track = ActiveTrack::default();
        let mut session = begin(&base, 1.0);
        session.extend(&[sample(20.0, 4)]).expect("extend");
        let drawn = track.advance(&base, Some(session.active()), true);
        let cancelled = track.advance(&session.clone().cancel(), None, true);
        assert_eq!(cancelled.redraw, drawn.fresh_region);
        track.advance(&base, Some(session.active()), true);
        let committed = track.advance(&session.finish().artwork, None, true);
        assert_eq!(committed.redraw, None);
    }

    #[test]
    fn samples_committed_before_their_evaluation_are_redrawn() {
        let base = blank();
        let mut track = ActiveTrack::default();
        let mut session = begin(&base, 1.0);
        session.extend(&[sample(20.0, 4)]).expect("extend");
        track.advance(&base, Some(session.active()), true);
        let tail = [sample(30.0, 8), sample(40.0, 12)];
        session.extend(&tail).expect("extend");
        let style = *session.active().style();
        let committed = track.advance(&session.finish().artwork, None, true);
        let expected = stroke_region(&style, &[sample(20.0, 4), tail[0], tail[1]]);
        assert_eq!(committed.redraw, expected);
    }

    #[test]
    fn another_stroke_starts_over_and_erases_the_first() {
        let base = blank();
        let mut track = ActiveTrack::default();
        let first = begin(&base, 1.0);
        let drawn = track.advance(&base, Some(first.active()), true);
        let second = begin(&base, 40.0);
        let next = track.advance(&base, Some(second.active()), true);
        assert!(next.reset);
        assert_eq!(next.redraw, drawn.fresh_region);
        let hidden = track.advance(&base, Some(second.active()), false);
        assert_eq!(hidden.fresh, 0..0);
        assert!(
            !track
                .advance(&base, Some(second.active()), true)
                .fresh
                .is_empty()
        );
    }
}
