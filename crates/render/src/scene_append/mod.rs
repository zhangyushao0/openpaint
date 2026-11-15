//! Strokes appended to a kept scene in place. The lift commits a version that is the one before
//! with the stroke appended on its place (its mirrored copies too); built anew, its scene is the
//! scene before with that stroke's items right before the place's and every item after them moved
//! by their count. So the kept scene takes them there: its items, its raster units, where its
//! strokes' spans and its smudges lie, the place's layer before it. A build costs what the whole
//! artwork holds (41,000 items over the A4 test page, 22 to 41 ms on the tablet); this costs the
//! stroke and what lies above it.

use std::ops::Range;

use openpaint_core::{Artwork, LayerId, Stroke, StrokeList, StrokeTool};

use crate::raster_scene::stroke_unit;
use crate::scene::Scene;
use crate::scene_content::stroke_end;
use crate::scene_reuse::span_items;

/// Whether `now` is `base` with strokes appended on the paint or mask `place` names and nothing
/// else changed: what lies under the place is what `base` holds, and the strokes appended (the
/// snapshots taken over `base` stay; its scene takes them in place).
pub(crate) fn appended_over(
    base: &Artwork,
    now: &Artwork,
    (layer, on_mask): (LayerId, bool),
) -> bool {
    let appended = |old: &StrokeList, new: &StrokeList| {
        new.len() > old.len()
            && old.last().is_none_or(|last| {
                (new.get(old.len() - 1)).is_some_and(|same| same.shares_samples(last))
            })
    };
    if base.canvas() != now.canvas()
        || base.groups() != now.groups()
        || base.layers().len() != now.layers().len()
    {
        return false;
    }
    base.layers().iter().zip(now.layers()).all(|(old, new)| {
        let drawn = old.id() == layer;
        let masks = match (old.mask(), new.mask()) {
            (None, None) => true,
            (Some(a), Some(b)) if drawn && on_mask => {
                a.enabled() == b.enabled()
                    && a.inverted() == b.inverted()
                    && a.edits() == b.edits()
                    && appended(a.strokes(), b.strokes())
            }
            (Some(a), Some(b)) => a.enabled() == b.enabled() && a.same_grey(b),
            _ => false,
        };
        // Edits compare shared masks and patches first: a commit keeps them.
        let paint = if drawn && !on_mask {
            old.edits() == new.edits() && appended(old.strokes(), new.strokes())
        } else {
            old.same_paint(new)
        };
        old.id() == new.id() && old.props() == new.props() && masks && paint
    })
}

/// The strokes `now` appends on `place` to `base` (`appended_over`), oldest first.
pub(crate) fn appended(base: &Artwork, now: &Artwork, place: (LayerId, bool)) -> Vec<Stroke> {
    let strokes = |artwork: &Artwork| {
        let layer = artwork.layer(place.0)?;
        match layer.mask().filter(|_| place.1) {
            Some(mask) => Some(mask.strokes().clone()),
            None => Some(layer.strokes().clone()),
        }
    };
    let (Some(old), Some(new)) = (strokes(base), strokes(now)) else {
        return Vec::new();
    };
    new.since(old.len()).cloned().collect()
}

impl Scene {
    /// `strokes` appended on the scene's place (its layer's paint or mask, each stroke drawn
    /// without a clip and not a smudge) laid into the scene as a build lays them; the items
    /// inserted. `None`, the scene unchanged, when it has no place or a stroke is not of that
    /// kind.
    pub(crate) fn append(&mut self, strokes: &[Stroke]) -> Option<Range<usize>> {
        let at = self.active_before.as_ref()?.end;
        let laid = |stroke: &Stroke| {
            stroke.clip().is_none() && stroke.style().tool() != StrokeTool::Smudge
        };
        if !strokes.iter().all(laid) {
            return None;
        }
        let mut items = Vec::new();
        let mut spans = Vec::new();
        for stroke in strokes {
            let laid = span_items(stroke);
            // A stroke without spans shows nothing: a build lists none of its items.
            if laid.is_empty() {
                continue;
            }
            let start = at + items.len();
            items.extend(laid);
            spans.push((stroke.clone(), start..at + items.len()));
            items.push(stroke_end(stroke, 0));
        }
        let count = items.len();
        self.items.splice(at..at, items);
        // A smudge of the place's is no longer its layer's newest stroke.
        let place = self.active_before.clone().unwrap_or_default();
        for smudge in (self.smudges.iter_mut()).filter(|smudge| place.contains(&smudge.item)) {
            smudge.newest &= strokes.is_empty();
        }
        self.moved(at, count);
        for (stroke, range) in spans {
            self.note_spans(&stroke, range.clone());
            self.unit_before_place(stroke_unit(self, range.clone(), range.end));
        }
        Some(at..at + count)
    }

    /// Every index of an item from `at` on moved `by` items later: the place's, the smudges', the
    /// adjustments' and filters' (they lie above the place), the spans', the units'.
    fn moved(&mut self, at: usize, by: usize) {
        let later = |index: &mut usize| {
            if *index >= at {
                *index += by;
            }
        };
        if let Some(before) = &mut self.active_before {
            later(&mut before.end);
        }
        for smudge in &mut self.smudges {
            later(&mut smudge.item);
            later(&mut smudge.before.start);
            later(&mut smudge.before.end);
        }
        for adjust in &mut self.adjusts {
            later(&mut adjust.item);
            later(&mut adjust.before.start);
            later(&mut adjust.before.end);
        }
        for filter in &mut self.filters {
            later(&mut filter.item);
            later(&mut filter.before.start);
            later(&mut filter.before.end);
        }
        self.spans.moved(at, by);
        for unit in &mut self.raster.units {
            unit.renumber(later);
        }
    }

    /// `unit` laid right before the place's.
    fn unit_before_place(&mut self, unit: crate::raster_scene::Unit) {
        if let Some(place) = self.raster.active() {
            self.raster.units.insert(place, unit);
        }
    }
}

#[cfg(test)]
mod tests;
