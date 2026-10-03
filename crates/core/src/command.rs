//! Artwork commands and the result of applying one.

use crate::artwork::Artwork;
use crate::error::CoreError;
use crate::id::LayerId;
use crate::layer_command;
use crate::value::LayerName;

/// Every structural edit of an artwork, one family per variant.
#[derive(Debug, Clone, PartialEq)]
pub enum ArtworkCommand {
    Layer(LayerCommand),
}

/// Layer structure and properties.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerCommand {
    /// Adds an empty layer directly above the active one and makes it active.
    Create,
    /// Removes a layer; if it was active, the layer below it (or the new bottom) becomes active.
    Delete(LayerId),
    /// Moves a layer to `index`, counted from the bottom after the move.
    Reorder {
        layer: LayerId,
        index: usize,
    },
    /// Makes a layer the target of new strokes.
    Select(LayerId),
    Rename {
        layer: LayerId,
        name: LayerName,
    },
    SetVisible {
        layer: LayerId,
        visible: bool,
    },
    /// `opacity` in `0..=1`.
    SetOpacity {
        layer: LayerId,
        opacity: f32,
    },
    SetLocked {
        layer: LayerId,
        locked: bool,
    },
    /// Alpha Lock: strokes begun from now on paint only over the layer's existing paint.
    /// Strokes drawn before keep how they were drawn.
    SetAlphaLocked {
        layer: LayerId,
        locked: bool,
    },
}

/// What an edit changed, so callers can decide on history, saving and redrawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Nothing; `Applied::artwork` is the same version as the input.
    None,
    /// Only the active layer moved.
    ActiveLayer,
    /// Layers, their properties or their strokes changed.
    Content,
}

/// The outcome of an edit: the new version and the kind of change.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub artwork: Artwork,
    pub change: Change,
}

impl Applied {
    pub(crate) fn unchanged(artwork: &Artwork) -> Self {
        Self {
            artwork: artwork.clone(),
            change: Change::None,
        }
    }
}

impl Artwork {
    /// Applies one command and returns the new version; `self` is unchanged.
    ///
    /// # Errors
    /// `UnknownLayer` for an id not in the artwork; `LayerLimit`, `LastLayer`,
    /// `IndexOutOfRange` and `InvalidValue` as documented on each command.
    pub fn apply(&self, command: ArtworkCommand) -> Result<Applied, CoreError> {
        match command {
            ArtworkCommand::Layer(command) => layer_command::apply(self, command),
        }
    }
}
