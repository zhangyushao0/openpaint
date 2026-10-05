//! `StrokeList`: a layer's strokes in drawing order, shared between artwork versions in chunks.

use std::sync::Arc;

use crate::id::IdCounters;
use crate::stroke::Stroke;

/// Strokes per chunk. Appending a stroke copies only the last chunk and the chunk table.
const CHUNK: usize = 256;

/// A layer's strokes, oldest first. Clone is O(1); versions share all full chunks.
#[derive(Debug, Clone)]
pub struct StrokeList {
    chunks: Arc<[Arc<[Stroke]>]>,
    len: usize,
}

impl StrokeList {
    pub(crate) fn from_strokes(strokes: &[Stroke]) -> Self {
        Self {
            chunks: strokes.chunks(CHUNK).map(Arc::from).collect(),
            len: strokes.len(),
        }
    }

    /// A new list with `stroke` appended; `self` is unchanged.
    pub(crate) fn pushed(&self, stroke: Stroke) -> Self {
        let mut chunks = self.chunks.to_vec();
        match chunks.last_mut() {
            Some(last) if last.len() < CHUNK => {
                let mut grown = last.to_vec();
                grown.push(stroke);
                *last = Arc::from(grown);
            }
            _ => chunks.push(Arc::from([stroke])),
        }
        Self {
            chunks: Arc::from(chunks),
            len: self.len + 1,
        }
    }

    /// The same strokes under new ids from `ids`, in order (a duplicated layer's).
    pub(crate) fn renumbered(&self, ids: &mut IdCounters) -> Self {
        let strokes: Vec<Stroke> = self
            .iter()
            .map(|stroke| stroke.renumbered(ids.take_stroke()))
            .collect();
        Self::from_strokes(&strokes)
    }

    /// The very same list, shared (O(1)); equal lists built apart are not the same.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.chunks, &other.chunks)
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The newest stroke.
    #[must_use]
    pub fn last(&self) -> Option<&Stroke> {
        self.chunks.last().and_then(|chunk| chunk.last())
    }

    /// Oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Stroke> {
        self.chunks.iter().flat_map(|chunk| chunk.iter())
    }

    /// The strokes from position `start` on, oldest first; finding `start` is O(1).
    pub fn since(&self, start: usize) -> impl Iterator<Item = &Stroke> {
        let first = start / CHUNK;
        let skip = start % CHUNK;
        self.chunks
            .get(first..)
            .unwrap_or_default()
            .iter()
            .flat_map(|chunk| chunk.iter())
            .skip(skip)
    }

    /// The stroke at position `index`, oldest first; O(1).
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Stroke> {
        self.chunks.get(index / CHUNK)?.get(index % CHUNK)
    }

    /// Heap bytes this list holds that `base` does not share: the chunk table, every chunk not
    /// shared with `base`'s chunk at the same place, and the samples of strokes in those chunks
    /// whose samples `base` does not share. O(chunks) plus O(`CHUNK`) per unshared chunk.
    pub(crate) fn bytes_since(&self, base: Option<&Self>) -> usize {
        if base.is_some_and(|base| Arc::ptr_eq(&self.chunks, &base.chunks)) {
            return 0;
        }
        let mut bytes = size_of_val::<[Arc<[Stroke]>]>(&self.chunks);
        for (index, chunk) in self.chunks.iter().enumerate() {
            let old = base.and_then(|base| base.chunks.get(index));
            if old.is_some_and(|old| Arc::ptr_eq(old, chunk)) {
                continue;
            }
            bytes += size_of_val::<[Stroke]>(chunk);
            for (place, stroke) in chunk.iter().enumerate() {
                let shared = old
                    .and_then(|old| old.get(place))
                    .is_some_and(|old| stroke.shares_samples(old));
                if !shared {
                    bytes += size_of_val(stroke.samples());
                }
            }
        }
        bytes
    }
}

impl Default for StrokeList {
    fn default() -> Self {
        Self::from_strokes(&[])
    }
}

impl PartialEq for StrokeList {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}
