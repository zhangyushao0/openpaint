//! `CoreError`: every way an artwork edit, a value or a stored artwork can be rejected.

use std::fmt;

use crate::id::{GroupId, LayerId};
use crate::merge::Unmergeable;

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
    /// No group with this id exists in the artwork.
    UnknownGroup(GroupId),
    /// Groups are one level deep: this layer, or one it would group, already lies in a group.
    GroupLayered(LayerId),
    /// The layer is locked; its content cannot change.
    LayerLocked(LayerId),
    /// The layer's alpha is locked; the eraser cannot take paint away.
    AlphaLocked(LayerId),
    /// The layer has no mask.
    NoMask(LayerId),
    /// The layer's mask is disabled; nothing may be painted on it, inverted or cleared.
    MaskOff(LayerId),
    /// The active layer's mask is the target, and this edit acts on paint only (transform).
    OnMask(LayerId),
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
    /// The layer cannot merge into the one below it (Merge Down) without changing what shows.
    Unmergeable { layer: LayerId, why: Unmergeable },
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue { what } => write!(f, "invalid {what}"),
            Self::UnknownLayer(id) => write!(f, "no layer with id {}", id.get()),
            Self::LastLayer => f.write_str("the last layer cannot be deleted"),
            Self::LayerLimit { max } => write!(f, "an artwork holds at most {max} layers"),
            Self::UnknownGroup(id) => write!(f, "no group with id {}", id.get()),
            Self::GroupLayered(id) => write!(f, "layer {} already lies in a group", id.get()),
            Self::LayerLocked(id) => write!(f, "layer {} is locked", id.get()),
            Self::AlphaLocked(id) => write!(f, "layer {}'s alpha is locked", id.get()),
            Self::NoMask(id) => write!(f, "layer {} has no mask", id.get()),
            Self::MaskOff(id) => write!(f, "layer {}'s mask is off", id.get()),
            Self::OnMask(id) => write!(f, "layer {}'s mask is the target", id.get()),
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
            Self::Unmergeable { layer, why } => {
                write!(f, "layer {} cannot merge down: {why:?}", layer.get())
            }
        }
    }
}

impl std::error::Error for CoreError {}
