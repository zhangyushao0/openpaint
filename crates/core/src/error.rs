//! `CoreError`: every way an artwork edit, a value or a stored artwork can be rejected.

use std::fmt;

use crate::id::LayerId;

/// Why core refused a value, a command, a stroke step or a stored artwork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// A value is not finite or outside its documented range; `what` names the value.
    InvalidValue { what: &'static str },
    /// No layer with this id exists in the artwork.
    UnknownLayer(LayerId),
    /// An artwork always keeps at least one layer.
    LastLayer,
    /// The artwork already holds the maximum number of layers.
    LayerLimit { max: usize },
    /// The layer is locked; its content cannot change.
    LayerLocked(LayerId),
    /// A layer position outside `0..len`.
    IndexOutOfRange { index: usize, len: usize },
    /// Stroke sample times must never go backwards.
    SampleOrder { previous_ms: u32, next_ms: u32 },
    /// A stroke may hold at most `max` samples.
    SampleLimit { max: usize },
    /// A stored artwork has no layers.
    NoLayers,
    /// A stored stroke has no samples.
    EmptyStroke,
    /// A stored id appears twice; `what` names the id kind.
    DuplicateId { what: &'static str, id: u64 },
    /// A stored id is zero or not below its counter, so a later edit could reuse it.
    IdOutOfRange { what: &'static str, id: u64 },
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue { what } => write!(f, "invalid {what}"),
            Self::UnknownLayer(id) => write!(f, "no layer with id {}", id.get()),
            Self::LastLayer => f.write_str("the last layer cannot be deleted"),
            Self::LayerLimit { max } => write!(f, "an artwork holds at most {max} layers"),
            Self::LayerLocked(id) => write!(f, "layer {} is locked", id.get()),
            Self::IndexOutOfRange { index, len } => {
                write!(f, "layer position {index} is outside 0..{len}")
            }
            Self::SampleOrder {
                previous_ms,
                next_ms,
            } => write!(f, "sample time {next_ms} ms is before {previous_ms} ms"),
            Self::SampleLimit { max } => write!(f, "a stroke holds at most {max} samples"),
            Self::NoLayers => f.write_str("the artwork has no layers"),
            Self::EmptyStroke => f.write_str("a stroke has no samples"),
            Self::DuplicateId { what, id } => write!(f, "{what} {id} appears twice"),
            Self::IdOutOfRange { what, id } => write!(f, "{what} {id} is out of range"),
        }
    }
}

impl std::error::Error for CoreError {}
