//! `StrokeList`: a layer's strokes in drawing order, shared between artwork versions in chunks.

use std::sync::Arc;

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

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Stroke> {
        self.chunks.iter().flat_map(|chunk| chunk.iter())
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
