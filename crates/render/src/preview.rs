//! Adjustment previews (S35). While a slider moves, every new value is a version that differs
//! from the one shown only in that adjustment's values (`retuned`): the scene keeps its items
//! and changes those ops in place, so no value builds a scene. Since S65 (DECISIONS D92) the
//! raster core draws each value where the adjustment changes pixels, from the scene as the
//! committed adjustment's (no snapshot of the layer before it), so a value previewed shows the
//! pixels the committed adjustment shows.

use openpaint_core::{AdjustEdit, Artwork, RegionEdit, StrokeList};

use crate::scene_adjust::EditPlace;

/// A preview's next value: the adjustment at `place` now has `edit`'s values.
#[derive(Debug, Clone)]
pub(crate) struct Retune {
    pub(crate) place: EditPlace,
    pub(crate) edit: AdjustEdit,
}

/// How `after` differs from `before` when only one adjustment's values changed (its kind,
/// clip and place kept): everything else the same content, shared.
pub(crate) fn retuned(before: &Artwork, after: &Artwork) -> Option<Retune> {
    if before.same_version(after)
        || before.canvas() != after.canvas()
        || before.layers().len() != after.layers().len()
        || before.groups() != after.groups()
    {
        return None;
    }
    let mut found = Vec::new();
    for (old, new) in before.layers().iter().zip(after.layers()) {
        if old.id() != new.id()
            || old.props() != new.props()
            || !same_strokes(old.strokes(), new.strokes())
        {
            return None;
        }
        for (index, edit) in differing(old.edits(), new.edits())? {
            found.push(((new.id(), false, index), edit));
        }
        match (old.mask(), new.mask()) {
            (None, None) => {}
            (Some(a), Some(b))
                if (a.enabled(), a.inverted()) == (b.enabled(), b.inverted())
                    && same_strokes(a.strokes(), b.strokes()) =>
            {
                for (index, edit) in differing(a.edits(), b.edits())? {
                    found.push(((new.id(), true, index), edit));
                }
            }
            _ => return None,
        }
    }
    match found.as_slice() {
        [(place, [_, new])] => Some(Retune {
            place: *place,
            edit: (*new).clone(),
        }),
        _ => None,
    }
}

/// The same strokes in both lists (they only grow, so sharing the last one's samples means
/// sharing all).
fn same_strokes(a: &StrokeList, b: &StrokeList) -> bool {
    a.len() == b.len()
        && match (a.last(), b.last()) {
            (Some(x), Some(y)) => x.shares_samples(y),
            _ => true,
        }
}

/// The edits of `b` that differ from `a`'s at their index in an adjustment's values only;
/// `None` when the lists differ otherwise.
fn differing<'a>(
    a: &'a [RegionEdit],
    b: &'a [RegionEdit],
) -> Option<Vec<(usize, [&'a AdjustEdit; 2])>> {
    if a.len() != b.len() {
        return None;
    }
    let mut out = Vec::new();
    for (index, (x, y)) in a.iter().zip(b).enumerate() {
        if x.same(y) {
            continue;
        }
        let (RegionEdit::Adjust(x), RegionEdit::Adjust(y)) = (x, y) else {
            return None;
        };
        let clip = match (x.clip(), y.clip()) {
            (Some(p), Some(q)) => p.shares(q),
            (None, None) => true,
            _ => false,
        };
        let kind = x.adjustment().kind() == y.adjustment().kind();
        if !clip || !kind || x.after() != y.after() || x.inverted() != y.inverted() {
            return None;
        }
        out.push((index, [x, y]));
    }
    Some(out)
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
