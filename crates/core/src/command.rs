//! Artwork commands and the result of applying one.

use crate::artwork::Artwork;
use crate::blend::BlendMode;
use crate::error::CoreError;
use crate::id::{GroupId, LayerId};
use crate::value::LayerName;
use crate::{group_command, layer_command, mask_command};

/// Every structural edit of an artwork, one family per variant.
#[derive(Debug, Clone, PartialEq)]
pub enum ArtworkCommand {
    Layer(LayerCommand),
    Group(GroupCommand),
    Mask(MaskCommand),
}

/// Layer masks (S24). Every one but `Select` acts on a layer that is not locked
/// (`LayerLocked`) and, but `Add`, has a mask (`NoMask`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskCommand {
    /// Gives the layer a white mask (nothing changes on the canvas) and makes it the active
    /// layer with its mask the target. A layer with a mask keeps it: `Change::None`.
    Add(LayerId),
    /// Makes the layer active with its mask the target of strokes and fills; no edit.
    /// `MaskOff` for a disabled mask.
    Select(LayerId),
    /// A disabled mask keeps what was painted on it and shows the whole layer; it is never
    /// the target (disabling the target makes the layer's paint the target).
    SetEnabled { layer: LayerId, enabled: bool },
    /// Black for white and white for black, from now on. `MaskOff` while disabled.
    Invert(LayerId),
    /// Back to white everywhere, not inverted. `MaskOff` while disabled.
    Clear(LayerId),
    /// The mask goes; the layer shows whole. Its paint becomes the target.
    Delete(LayerId),
}

/// Layer structure and properties.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerCommand {
    /// Adds an empty layer directly above the active one, in the active one's group, and makes
    /// it active.
    Create,
    /// Removes a layer; if it was active, the layer below it (or the new bottom) becomes active.
    /// A group losing its last layer goes with it.
    Delete(LayerId),
    /// Moves a layer to `index`, counted from the bottom after the move. It lands in the group
    /// of the layers on both sides when they share one, stays in its own group when it lands
    /// beside it, and is in no group otherwise; a group is never split.
    Reorder {
        layer: LayerId,
        index: usize,
    },
    /// One step up: the top layer of a group leaves it (it stays where it is, above the group);
    /// a layer in no group right under a group joins it as its bottom layer; any other layer
    /// swaps places with the one above. `IndexOutOfRange` at the top of the stack outside a
    /// group.
    Raise(LayerId),
    /// `Raise` downwards: a group's bottom layer leaves it under it, a layer in no group right
    /// above a group joins it as its top layer, any other swaps with the one below.
    Lower(LayerId),
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
    /// Clipping Mask on or off (`LayerProps::clipping`).
    SetClipping {
        layer: LayerId,
        clipping: bool,
    },
    SetBlend {
        layer: LayerId,
        blend: BlendMode,
    },
}

/// Groups: making, unmaking, showing and fading them. Groups are one level deep.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupCommand {
    /// Puts `layer`, which lies in no group, into a new group ("Group n") with the layer under
    /// it. Clipping chains stay whole: when `layer` is in a chain (a base with layers clipping
    /// to it, or one of those), the group holds that chain alone; otherwise it holds `layer`
    /// and the chain or layer right under it, or `layer` alone at the bottom of the stack
    /// (`GroupLayered` when either lies in a group).
    Create {
        layer: LayerId,
    },
    /// A new empty layer in a new group above the active layer and its chain (above its group
    /// when it lies in one); the new layer becomes active.
    New,
    /// Removes the group; its layers stay where they are, hidden when the group was hidden.
    Ungroup(GroupId),
    SetVisible {
        group: GroupId,
        visible: bool,
    },
    /// `opacity` in `0..=1`.
    SetOpacity {
        group: GroupId,
        opacity: f32,
    },
}

/// What an edit changed, so callers can decide on history, saving and redrawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Nothing; `Applied::artwork` is the same version as the input.
    None,
    /// Only the active layer, or whether its mask is the target, moved.
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
    /// `UnknownLayer` or `UnknownGroup` for an id not in the artwork; `LayerLimit`, `LastLayer`,
    /// `IndexOutOfRange`, `GroupLayered` and `InvalidValue` as documented on each command.
    pub fn apply(&self, command: ArtworkCommand) -> Result<Applied, CoreError> {
        match command {
            ArtworkCommand::Layer(command) => layer_command::apply(self, command),
            ArtworkCommand::Group(command) => group_command::apply(self, command),
            ArtworkCommand::Mask(command) => mask_command::apply(self, command),
        }
    }
}
