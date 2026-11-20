//! What the user can ask for whose availability depends on the session, and why it may not run
//! now.

use std::fmt;

use crate::error::AppError;

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
    /// Choose, show, hide, fade, clip or blend a layer; show, hide, fade or undo its group.
    EditLayer,
    /// Group the layer with the one under it.
    GroupLayer,
    /// Give the layer a mask (it has none).
    AddMask,
    /// Turn the layer's mask on or off, or delete it.
    EditMask,
    /// Paint on, invert or clear the layer's mask (it is on).
    ChangeMask,
    /// Merge the layer into the one below it (`Artwork::mergeable`; `LayerList::merge_down`
    /// says why not).
    MergeDown,
    /// Leave the canvas for the gallery, or act in the gallery.
    Gallery,
    /// Export the artwork to a file or another app.
    Export,
    /// Choose a reference image with the system's picker.
    ChooseReference,
    /// Choose a picture with the system's picker to insert as a new layer.
    InsertImage,
    /// Fill a region with the colour (ColorDrop).
    Fill,
    /// Move, scale or turn the layer's content or the selection's.
    Transform,
    /// Adjust the layer's colours, blur or sharpen it.
    Adjust,
    /// Cut, copy or paste, as the copy menu offers.
    Clipboard(crate::clipboard::ClipboardAction),
    /// Open the replay of the artwork (S163): nothing to replay without a record.
    Replay,
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

    /// Whether a control for the action looks usable: available now, or only blocked while a
    /// stroke is drawn, when a tap is refused without any visual change (ARCHITECTURE section
    /// 10). Nothing greys out or flickers per stroke.
    #[must_use]
    pub const fn is_offered(self) -> bool {
        matches!(self, Self::Available | Self::StrokeInProgress)
    }

    /// Nothing while available; otherwise `action` refused for this reason.
    pub(crate) const fn require(self, action: EditorAction) -> Result<(), AppError> {
        if self.is_available() {
            Ok(())
        } else {
            Err(AppError::Unavailable {
                action,
                availability: self,
            })
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
            Self::EditLayer => "change the layer",
            Self::GroupLayer => "group the layer",
            Self::AddMask => "add a mask",
            Self::EditMask => "change the mask",
            Self::ChangeMask => "paint on the mask",
            Self::MergeDown => "merge the layer down",
            Self::Gallery => "use the gallery",
            Self::Export => "export the artwork",
            Self::ChooseReference => "choose a reference image",
            Self::InsertImage => "insert an image",
            Self::Fill => "fill",
            Self::Transform => "transform",
            Self::Adjust => "adjust",
            Self::Clipboard(_) => "cut, copy or paste",
            Self::Replay => "replay the artwork",
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
