//! A finished stroke's span items kept from one scene to the next (S56). A stroke's spans (its
//! `Segment` ops and their bounds) depend on its style and samples alone: not on where it lies in
//! the scene (a span names its paint op by how far after it that lies), its layer, clip or the
//! view it is seen in. Building a scene computed them again for every stroke (the curve, the
//! dynamics at every knot, the stamps' placement): on the tablet 18 to 37 ms at the pen-down and
//! at the lift over a few hundred strokes. Now a scene notes where each stroke's spans lie in its
//! items, and the next scene built on the same target copies them from there; only a stroke the
//! scene before did not draw has its spans computed.

use std::collections::HashMap;
use std::ops::Range;

use openpaint_core::Stroke;

use crate::geom::Bounds;
use crate::protocol::Op;
use crate::scene::{Item, Scene};
use crate::stroke::segments;

/// Where each finished stroke's span items lie in a scene, by the address of its samples, with
/// the stroke: holding its samples, so no other stroke's take that address while it is noted.
#[derive(Debug, Default)]
pub(crate) struct Spans {
    at: HashMap<usize, (Stroke, Range<usize>)>,
}

/// A scene's items and spans, handed to the next scene built so it copies spans from them.
#[derive(Debug, Default)]
pub(crate) struct Earlier {
    items: Vec<Item>,
    spans: Spans,
}

impl Scene {
    /// This scene's items and spans for the next scene to copy from; this scene is left without
    /// them (it is being replaced).
    pub(crate) fn take_earlier(&mut self) -> Earlier {
        Earlier {
            items: std::mem::take(&mut self.items),
            spans: std::mem::take(&mut self.spans),
        }
    }

    /// Pushes the span items of `stroke` (not a smudge), copied from the scene before when it
    /// drew the same stroke, else computed; returns where they lie.
    pub(crate) fn push_spans(&mut self, stroke: &Stroke) -> Range<usize> {
        let Self { earlier, items, .. } = self;
        let start = items.len();
        let kept = earlier
            .spans
            .at
            .get(&key(stroke))
            .filter(|(kept, _)| kept.shares_samples(stroke) && kept.style() == stroke.style());
        if let Some((_, range)) = kept {
            items.extend_from_slice(&earlier.items[range.clone()]);
            return start..items.len();
        }
        let spans: Vec<_> = segments(stroke.style(), stroke.samples()).collect();
        // Each span's paint op is the stroke's end, right after its last span.
        let parts: Vec<(Op, Bounds)> = (1..=spans.len() as i32)
            .rev()
            .zip(&spans)
            .map(|(paint, segment)| (segment.op(paint), segment.bounds()))
            .collect();
        items.extend(
            parts
                .into_iter()
                .map(|(op, bounds)| Item::Segment { op, bounds }),
        );
        start..items.len()
    }

    /// Notes that `stroke`'s span items lie at `range`, unless they were noted already (a stroke
    /// a moved selection shows twice).
    pub(crate) fn note_spans(&mut self, stroke: &Stroke, range: Range<usize>) {
        self.spans
            .at
            .entry(key(stroke))
            .or_insert_with(|| (stroke.clone(), range));
    }

    /// The scene before is no longer needed once this one is built.
    pub(crate) fn forget_earlier(&mut self) {
        self.earlier = Earlier::default();
    }
}

/// The address of `stroke`'s samples.
fn key(stroke: &Stroke) -> usize {
    stroke.samples().as_ptr().addr()
}
