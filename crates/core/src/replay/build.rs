//! Making the drawing order: every list of records the artwork holds (a layer's paint and its
//! mask, a merged layer's content and its mask, at any depth) in a pre-order walk (layers bottom
//! to top, a layer's paint, the merged records it holds, then its mask), and every record's
//! step, sorted by its place (see the module).

use std::sync::Arc;

use crate::artwork::Artwork;
use crate::id::LayerId;
use crate::region::RegionEdit;
use crate::replay::{EditKind, Step, StepKind};
use crate::stroke_list::StrokeList;

/// The artwork's lists and where each step's record lies in them.
#[derive(Debug)]
pub(super) struct Lists {
    /// In the walk's order: a list before those it holds.
    pub(super) lists: Vec<List>,
    /// Per layer, by its position: its paint's list and its mask's.
    pub(super) layers: Vec<(usize, Option<usize>)>,
    /// Per step: its list and its record there.
    pub(super) records: Vec<(usize, Record)>,
}

/// A record in its list: a stroke or an edit, by its index there.
#[derive(Debug, Clone, Copy)]
pub(super) enum Record {
    Stroke(usize),
    Edit(usize),
}

/// One list of records, as the finished artwork holds it.
#[derive(Debug)]
pub(super) struct List {
    pub(super) strokes: StrokeList,
    pub(super) edits: Arc<[RegionEdit]>,
    /// Each stroke's step, in the list's order (growing: ids grow along a list).
    pub(super) stroke_steps: Vec<usize>,
    /// Each edit's step, or a merged record's content's lists.
    pub(super) edit_steps: Vec<EditStep>,
    /// The first step it holds at any depth; `usize::MAX` for none.
    pub(super) first: usize,
    /// A layer's paint or mask (its position, on its mask): its strokes are drawn live.
    pub(super) place: Option<(usize, bool)>,
}

/// Where an edit's step lies.
#[derive(Debug, Clone, Copy)]
pub(super) enum EditStep {
    Own(usize),
    /// A merged layer's record: its content's list and its mask's.
    Nested {
        paint: usize,
        mask: Option<usize>,
    },
}

/// A record before sorting: its place, then edits before strokes, then the walk's order.
struct Entry {
    key: (u64, bool, usize, usize),
    step: Step,
    list: usize,
    record: Record,
}

/// What every record of one list shares.
#[derive(Debug, Clone, Copy)]
struct Context {
    layer: LayerId,
    shows: bool,
    place: Option<(usize, bool)>,
}

/// The steps of `artwork` in order and its lists.
pub(super) fn order(artwork: &Artwork) -> (Vec<Step>, Lists) {
    let (mut lists, mut entries, mut layers) = (Vec::new(), Vec::new(), Vec::new());
    for (index, layer) in artwork.layers().iter().enumerate() {
        let shown = artwork.shown(layer.id);
        let paint = Context {
            layer: layer.id,
            shows: shown,
            place: Some((index, false)),
        };
        let paint = add(
            &mut lists,
            &mut entries,
            (&layer.strokes, &layer.edits),
            paint,
        );
        let mask = layer.mask.as_ref().map(|mask| {
            let context = Context {
                layer: layer.id,
                shows: shown && mask.enabled,
                place: Some((index, true)),
            };
            add(
                &mut lists,
                &mut entries,
                (&mask.strokes, &mask.edits),
                context,
            )
        });
        layers.push((paint, mask));
    }
    entries.sort_unstable_by_key(|entry| entry.key);
    let mut steps = Vec::with_capacity(entries.len());
    let mut records = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        let list = &mut lists[entry.list];
        match entry.record {
            Record::Stroke(at) => list.stroke_steps[at] = index,
            Record::Edit(at) => list.edit_steps[at] = EditStep::Own(index),
        }
        steps.push(entry.step);
        records.push((entry.list, entry.record));
    }
    // A list holds only lists after it: the walk's order backwards sees them first.
    for index in (0..lists.len()).rev() {
        let list = &lists[index];
        let strokes = list.stroke_steps.iter().copied().min();
        let edits = list.edit_steps.iter().map(|step| match *step {
            EditStep::Own(step) => step,
            EditStep::Nested { paint, mask } => {
                let mask = mask.map_or(usize::MAX, |mask| lists[mask].first);
                lists[paint].first.min(mask)
            }
        });
        let first = edits.chain(strokes).min().unwrap_or(usize::MAX);
        lists[index].first = first;
    }
    let lists = Lists {
        lists,
        layers,
        records,
    };
    (steps, lists)
}

/// Adds the list of `strokes` and `edits` and, depth first, the merged records it holds;
/// returns its index.
fn add(
    lists: &mut Vec<List>,
    entries: &mut Vec<Entry>,
    (strokes, edits): (&StrokeList, &Arc<[RegionEdit]>),
    context: Context,
) -> usize {
    let index = lists.len();
    lists.push(List {
        strokes: strokes.clone(),
        edits: Arc::clone(edits),
        stroke_steps: vec![0; strokes.len()],
        edit_steps: Vec::new(),
        first: usize::MAX,
        place: context.place,
    });
    add_strokes(entries, (strokes, index), context);
    let mut edit_steps = Vec::with_capacity(edits.len());
    for (at, edit) in edits.iter().enumerate() {
        let kind = match edit {
            RegionEdit::Nested(nested) => {
                let inner = Context {
                    place: None,
                    ..context
                };
                let layer = nested.layer();
                let paint = add(lists, entries, (&layer.strokes, &layer.edits), inner);
                let mask = (layer.mask.as_ref())
                    .map(|mask| add(lists, entries, (&mask.strokes, &mask.edits), inner));
                edit_steps.push(EditStep::Nested { paint, mask });
                continue;
            }
            RegionEdit::Fill(_) => EditKind::Fill,
            RegionEdit::Clear(_) => EditKind::Clear,
            RegionEdit::Transform(_) => EditKind::Transform,
            RegionEdit::Adjust(_) => EditKind::Adjust,
        };
        edit_steps.push(EditStep::Own(usize::MAX));
        entries.push(Entry {
            key: (edit.before_stroke(), false, index, at),
            step: Step {
                kind: StepKind::Edit(kind),
                layer: context.layer,
                shows: context.shows,
                live: false,
            },
            list: index,
            record: Record::Edit(at),
        });
    }
    lists[index].edit_steps = edit_steps;
    index
}

/// The steps of list `index`'s `strokes`, each at its id.
fn add_strokes(entries: &mut Vec<Entry>, (strokes, index): (&StrokeList, usize), context: Context) {
    for (at, stroke) in strokes.iter().enumerate() {
        let samples = stroke.samples();
        let first = samples.first().map_or(0, |sample| sample.time_ms());
        let last = samples.last().map_or(0, |sample| sample.time_ms());
        let kind = StepKind::Stroke {
            id: stroke.id(),
            began_ms: stroke.began_ms(),
            samples: samples.len(),
            drawn_ms: last.saturating_sub(first),
        };
        entries.push(Entry {
            key: (stroke.id().get(), true, index, at),
            step: Step {
                kind,
                layer: context.layer,
                shows: context.shows,
                live: context.place.is_some(),
            },
            list: index,
            record: Record::Stroke(at),
        });
    }
}
