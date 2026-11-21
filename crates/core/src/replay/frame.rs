//! The artwork after a number of steps (`DrawingOrder::frame`): every list cut to the records of
//! the steps done. Each list's last cut is kept (`Cache`), so a list a frame leaves as it was is
//! the very same list (its `Arc`s) as the frame before, and a list holding all its records is the
//! finished artwork's own: the renderer then sees a stroke appended where one was, and the
//! finished artwork where every step is done.

use std::sync::Arc;

use crate::artwork::Artwork;
use crate::id::LayerId;
use crate::nested::{NestedEdit, NestedLayer};
use crate::region::RegionEdit;
use crate::replay::build::{EditStep, Lists};
use crate::stroke_list::StrokeList;

/// The last cut of each list and the artwork they made, with the place it was made for.
#[derive(Debug, Clone, Default)]
pub(super) struct Cache {
    cuts: Vec<Option<Cut>>,
    artwork: Option<Artwork>,
    place: Option<(LayerId, bool)>,
}

/// One list cut to the steps done.
#[derive(Debug, Clone)]
struct Cut {
    strokes: StrokeList,
    edits: Arc<[RegionEdit]>,
    /// The merged layer this content and its mask's last cut make (a merged layer's content).
    nested: Option<Arc<NestedLayer>>,
}

/// `finished` after the steps before `done`, sharing what `cache` holds of the frames before;
/// its active layer (and whether its mask) `place`, the next stroke's, when given.
pub(super) fn artwork(
    finished: &Artwork,
    lists: &Lists,
    (done, place): (usize, Option<(LayerId, bool)>),
    cache: &mut Cache,
) -> Artwork {
    cache.cuts.resize(lists.lists.len(), None);
    let mut state = finished.state().clone();
    let mut changed = cache.place != place;
    if let Some((layer, on_mask)) = place
        && let Some(held) = state.layers.iter().find(|held| held.id == layer)
    {
        let masked = held.mask.as_ref().is_some_and(|mask| mask.enabled);
        state.active = layer;
        state.on_mask = on_mask && masked;
    }
    cache.place = place;
    for (index, &(paint, mask)) in lists.layers.iter().enumerate() {
        let (content, fresh) = cut(lists, paint, done, cache);
        changed |= fresh;
        let layer = &mut state.layers[index];
        (layer.strokes, layer.edits) = (content.strokes, content.edits);
        if let (Some(mask), Some(held)) = (mask, layer.mask.as_mut()) {
            let (content, fresh) = cut(lists, mask, done, cache);
            changed |= fresh;
            (held.strokes, held.edits) = (content.strokes, content.edits);
        }
    }
    if !changed && let Some(artwork) = &cache.artwork {
        return artwork.clone();
    }
    let artwork = Artwork::from_state(state);
    cache.artwork = Some(artwork.clone());
    artwork
}

/// List `index` cut to the steps before `done`, and whether it differs from its last cut.
fn cut(lists: &Lists, index: usize, done: usize, cache: &mut Cache) -> (Cut, bool) {
    let list = &lists.lists[index];
    let kept = list.stroke_steps.partition_point(|step| *step < done);
    let mut edits = Vec::with_capacity(list.edits.len());
    for (edit, step) in list.edits.iter().zip(&list.edit_steps) {
        match (edit, *step) {
            (_, EditStep::Own(step)) if step < done => {
                edits.push(if edit.after() <= kept {
                    edit.clone()
                } else {
                    edit.at(kept, edit.before_stroke())
                });
            }
            (RegionEdit::Nested(nested), EditStep::Nested { paint, mask }) => {
                let first = mask.map_or(usize::MAX, |mask| lists.lists[mask].first);
                if lists.lists[paint].first.min(first) < done {
                    let layer = merged(lists, nested, (paint, mask), done, cache);
                    edits.push(RegionEdit::Nested(NestedEdit {
                        after: nested.after.min(kept),
                        before_stroke: nested.before_stroke,
                        layer,
                    }));
                }
            }
            _ => {}
        }
    }
    let held = cache.cuts[index].take();
    let same = |a: &[RegionEdit], b: &[RegionEdit]| {
        a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.same(b))
    };
    let (strokes, fresh_strokes) = match &held {
        Some(held) if held.strokes.len() == kept => (held.strokes.clone(), false),
        _ => (list.strokes.prefix(kept), true),
    };
    let (edits, fresh_edits) = match &held {
        Some(held) if same(&held.edits, &edits) => (Arc::clone(&held.edits), false),
        _ if same(&list.edits, &edits) => (Arc::clone(&list.edits), true),
        _ => (Arc::from(edits), true),
    };
    let fresh = fresh_strokes || fresh_edits;
    let cut = Cut {
        strokes,
        edits,
        nested: held.and_then(|held| held.nested).filter(|_| !fresh),
    };
    cache.cuts[index] = Some(cut.clone());
    (cut, fresh)
}

/// The merged layer of `nested` with its content (list `paint`) and its mask's (list `mask`)
/// cut to the steps before `done`: the record's own when they hold all of theirs, the last one
/// made when neither changed.
fn merged(
    lists: &Lists,
    nested: &NestedEdit,
    (paint, mask): (usize, Option<usize>),
    done: usize,
    cache: &mut Cache,
) -> Arc<NestedLayer> {
    let (content, mut fresh) = cut(lists, paint, done, cache);
    let masked = mask.map(|mask| cut(lists, mask, done, cache));
    fresh |= masked.as_ref().is_some_and(|(_, fresh)| *fresh);
    let original = &nested.layer;
    let whole = |strokes: &StrokeList, edits: &Arc<[RegionEdit]>, cut: &Cut| {
        cut.strokes.same(strokes) && Arc::ptr_eq(&cut.edits, edits)
    };
    let mask_whole = match (&original.mask, &masked) {
        (Some(held), Some((cut, _))) => whole(&held.strokes, &held.edits, cut),
        _ => true,
    };
    if mask_whole && whole(&original.strokes, &original.edits, &content) {
        return Arc::clone(original);
    }
    if !fresh && let Some(layer) = content.nested {
        return layer;
    }
    let mut layer = NestedLayer::clone(original);
    (layer.strokes, layer.edits) = (content.strokes, content.edits);
    if let (Some(held), Some((cut, _))) = (layer.mask.as_mut(), masked) {
        (held.strokes, held.edits) = (cut.strokes, cut.edits);
    }
    let layer = Arc::new(layer);
    if let Some(Some(cut)) = cache.cuts.get_mut(paint) {
        cut.nested = Some(Arc::clone(&layer));
    }
    layer
}
