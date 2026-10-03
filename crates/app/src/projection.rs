//! `SessionProjection`: what the UI may offer right now. The UI greys out what is unavailable
//! and never works out the rules itself.

use std::fmt;

use crate::layers::LayerList;
use crate::storage::StorageNotice;

/// Something the user can ask for whose availability depends on the session. The layer actions
/// are offered for the active layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    Undo,
    Redo,
    RetrySave,
    DismissNotice,
    AddLayer,
    DeleteLayer,
    RaiseLayer,
    LowerLayer,
    /// Choose, show, hide or fade a layer.
    EditLayer,
}

/// Whether an action can run now, and why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    /// There is nothing to act on: no earlier version to undo to, none to redo, no failed save
    /// to retry, no notice to close, no other layer to keep, no place further up or down.
    NothingLeft,
    /// A stroke is being drawn; history and layers wait until it is committed or cancelled, so
    /// the stroke can neither be lost nor land on a version the user no longer sees.
    StrokeInProgress,
    /// The work the action would start is already running (a save being retried).
    Running,
    /// No room for more: the artwork holds the most layers it may.
    Full,
}

impl Availability {
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// A snapshot of the session for one UI frame. The default is no open session: nothing is
/// available, nothing to tell and no layers.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionProjection {
    pub(crate) undo: Availability,
    pub(crate) redo: Availability,
    pub(crate) retry_save: Availability,
    pub(crate) dismiss_notice: Availability,
    /// Add, delete, raise, lower and edit, for the active layer.
    pub(crate) layer_actions: [Availability; 5],
    pub(crate) storage: StorageNotice,
    pub(crate) layers: Option<LayerList>,
}

/// The layer actions in the order of `SessionProjection::layer_actions`.
pub(crate) const LAYER_ACTIONS: [EditorAction; 5] = [
    EditorAction::AddLayer,
    EditorAction::DeleteLayer,
    EditorAction::RaiseLayer,
    EditorAction::LowerLayer,
    EditorAction::EditLayer,
];

impl SessionProjection {
    #[must_use]
    pub const fn availability(&self, action: EditorAction) -> Availability {
        match action {
            EditorAction::Undo => self.undo,
            EditorAction::Redo => self.redo,
            EditorAction::RetrySave => self.retry_save,
            EditorAction::DismissNotice => self.dismiss_notice,
            EditorAction::AddLayer => self.layer_actions[0],
            EditorAction::DeleteLayer => self.layer_actions[1],
            EditorAction::RaiseLayer => self.layer_actions[2],
            EditorAction::LowerLayer => self.layer_actions[3],
            EditorAction::EditLayer => self.layer_actions[4],
        }
    }

    /// What to tell the user about saving and opening.
    #[must_use]
    pub const fn storage(&self) -> StorageNotice {
        self.storage
    }

    /// The layers, once an artwork is open.
    #[must_use]
    pub const fn layers(&self) -> Option<&LayerList> {
        self.layers.as_ref()
    }
}

impl Default for SessionProjection {
    fn default() -> Self {
        Self {
            undo: Availability::NothingLeft,
            redo: Availability::NothingLeft,
            retry_save: Availability::NothingLeft,
            dismiss_notice: Availability::NothingLeft,
            layer_actions: [Availability::NothingLeft; 5],
            storage: StorageNotice::None,
            layers: None,
        }
    }
}

impl fmt::Display for EditorAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::RetrySave => "retry saving",
            Self::DismissNotice => "close the notice",
            Self::AddLayer => "add a layer",
            Self::DeleteLayer => "delete the layer",
            Self::RaiseLayer => "move the layer up",
            Self::LowerLayer => "move the layer down",
            Self::EditLayer => "change the layer",
        })
    }
}

impl fmt::Display for Availability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Available => "available",
            Self::NothingLeft => "nothing left",
            Self::StrokeInProgress => "a stroke is in progress",
            Self::Running => "already running",
            Self::Full => "the layer limit is reached",
        })
    }
}
