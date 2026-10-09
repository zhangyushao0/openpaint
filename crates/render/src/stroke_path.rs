//! `Path`: a stroke's knots (`curve::knots`), how far along its path each lies, and what follows
//! from them: its spans, the ones no later sample can change, and the taper at each knot (S44).
//! The path's length is measured along the chords between consecutive knots, so a knot's
//! distance from the start depends only on the knots before it and stays settled while the
//! stroke is drawn.

use openpaint_core::{StrokeSample, StrokeStyle};

use crate::curve::{self, knots, point};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Path {
    pub(crate) knots: Vec<usize>,
    /// Canvas pixels from the first knot to each knot, along the chords between them.
    along: Vec<f32>,
    /// The pen has lifted: the stroke's end is known, so its end taper applies.
    lifted: bool,
}

impl Path {
    /// The path of a stroke as it is drawn: its end not known yet, so not tapered (also the
    /// widest the stroke can be, for the pixels it may reach).
    pub(crate) fn of(samples: &[StrokeSample]) -> Self {
        let knots = knots(samples);
        let mut along = Vec::with_capacity(knots.len());
        let mut length = 0.0;
        let mut before: Option<[f32; 2]> = None;
        for knot in &knots {
            let here = point(&samples[*knot]);
            if let Some(before) = before {
                length += curve::len(curve::sub(here, before));
            }
            along.push(length);
            before = Some(here);
        }
        Self {
            knots,
            along,
            lifted: false,
        }
    }

    /// The path of a finished stroke: its end taper applies.
    pub(crate) fn lifted(samples: &[StrokeSample]) -> Self {
        Self {
            lifted: true,
            ..Self::of(samples)
        }
    }

    /// Its spans: one between each two consecutive knots; a lone knot is one, a dot.
    pub(crate) fn spans(&self) -> usize {
        self.knots
            .len()
            .saturating_sub(1)
            .max(usize::from(!self.knots.is_empty()))
    }

    /// Canvas pixels from the first knot to knot `k` (an index into `knots`).
    pub(crate) fn along(&self, k: usize) -> f32 {
        self.along[k]
    }

    /// The spans no later sample can change while the stroke is drawn: all but the last two,
    /// which depend on the last sample (a knot until the next one shows whether it is one).
    pub(crate) fn settled(&self) -> usize {
        self.knots.len().saturating_sub(3)
    }

    /// The first span that samples from `from` on can change: a sample decides whether the
    /// one before it is a knot, and a span depends on the knot before and after it.
    pub(crate) fn changed_from(&self, from: usize) -> usize {
        let before = self.knots.iter().take_while(|k| **k + 1 < from).count();
        before.saturating_sub(2).min(self.spans())
    }

    /// The radius factor of a stroke of `style` at knot `k` (an index into `knots`): its
    /// recipe's taper at that distance from the start and, once lifted, from the end. A dot (a
    /// lone knot) is never tapered.
    pub(crate) fn taper(&self, style: &StrokeStyle, k: usize) -> f32 {
        let taper = style.recipe().taper();
        if self.knots.len() < 2 || taper.is_none() {
            return 1.0;
        }
        let along = self.along[k];
        let left = self.lifted.then(|| self.length() - along);
        taper.factor(along, left, style.size())
    }

    /// The first span whose end lies within `length` pixels of the path's end.
    pub(crate) fn ending_within(&self, length: f32) -> usize {
        let end = self.length();
        let first = self.along.iter().position(|along| end - along < length);
        first.map_or(self.spans(), |k| k.saturating_sub(1).min(self.spans()))
    }

    fn length(&self) -> f32 {
        self.along.last().copied().unwrap_or(0.0)
    }
}
