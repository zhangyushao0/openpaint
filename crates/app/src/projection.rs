//! `SessionProjection`: what the UI may offer right now. The UI greys out what there is nothing
//! for (`Availability::is_offered`) and never works out the rules itself.

use std::fmt;

use openpaint_core::CanvasSize;

use crate::adjust::AdjustProjection;
use crate::clipboard::{ClipboardAction, paste};
use crate::fill::{DEFAULT_THRESHOLD, FillProjection};
use crate::gallery::GalleryProjection;
use crate::layers::LayerList;
use crate::selection::SelectionProjection;
use crate::storage::StorageNotice;
use crate::tool_intent::BrushProjection;
use crate::transfer::TransferProjection;
use crate::transform::TransformProjection;
use crate::view::CanvasView;

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
    /// Cut, copy or paste (S40), as the copy menu offers.
    Clipboard(ClipboardAction),
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
}

/// A snapshot of the session for one UI frame. The default is no open session: nothing is
/// available, nothing to tell and no layers.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionProjection {
    pub(crate) undo: Availability,
    pub(crate) redo: Availability,
    pub(crate) retry_save: Availability,
    pub(crate) dismiss_notice: Availability,
    /// Add, delete, edit, group, the mask's three and Merge Down, for the active layer.
    pub(crate) layer_actions: [Availability; 8],
    pub(crate) gallery: Availability,
    pub(crate) storage: StorageNotice,
    pub(crate) layers: Option<LayerList>,
    pub(crate) view: CanvasView,
    pub(crate) canvas: Option<CanvasSize>,
    pub(crate) drawing: bool,
    pub(crate) fill: FillProjection,
    /// Boxed: a mask and a path, most frames nothing.
    pub(crate) selection: Box<SelectionProjection>,
    /// Boxed: a frame, most frames nothing.
    pub(crate) transform: Box<TransformProjection>,
    /// Boxed: curves, most frames nothing.
    pub(crate) adjust: Box<AdjustProjection>,
    /// The copy menu's commands in `ClipboardAction::ALL` order.
    pub(crate) clipboard: [Availability; 6],
    pub(crate) brush: Option<BrushProjection>,
    /// Boxed: rarely more than a notice, but its errors are large.
    pub(crate) transfers: Option<Box<TransferProjection>>,
}

/// What the whole screen shows for one UI frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ScreenProjection {
    /// The library is opening; nothing can be done yet.
    #[default]
    Starting,
    /// Boxed: far larger than the others.
    Canvas(Box<SessionProjection>),
    Gallery(GalleryProjection),
}

/// The layer actions in the order of `SessionProjection::layer_actions`.
pub(crate) const LAYER_ACTIONS: [EditorAction; 8] = [
    EditorAction::AddLayer,
    EditorAction::DeleteLayer,
    EditorAction::EditLayer,
    EditorAction::GroupLayer,
    EditorAction::AddMask,
    EditorAction::EditMask,
    EditorAction::ChangeMask,
    EditorAction::MergeDown,
];

impl SessionProjection {
    #[must_use]
    pub const fn availability(&self, action: EditorAction) -> Availability {
        match action {
            EditorAction::Export | EditorAction::ChooseReference => match &self.transfers {
                Some(transfers) => transfers.available,
                None => Availability::NothingLeft,
            },
            // A picker or transfer running first, then room for the picture's layer.
            EditorAction::InsertImage => match &self.transfers {
                Some(transfers) if !transfers.available.is_available() => transfers.available,
                Some(_) => self.layer_actions[0],
                None => Availability::NothingLeft,
            },
            EditorAction::Undo => self.undo,
            EditorAction::Redo => self.redo,
            EditorAction::RetrySave => self.retry_save,
            EditorAction::DismissNotice => self.dismiss_notice,
            EditorAction::AddLayer => self.layer_actions[0],
            EditorAction::DeleteLayer => self.layer_actions[1],
            EditorAction::EditLayer => self.layer_actions[2],
            EditorAction::GroupLayer => self.layer_actions[3],
            EditorAction::AddMask => self.layer_actions[4],
            EditorAction::EditMask => self.layer_actions[5],
            EditorAction::ChangeMask => self.layer_actions[6],
            EditorAction::MergeDown => self.layer_actions[7],
            EditorAction::Gallery => self.gallery,
            EditorAction::Clipboard(action) => self.clipboard[action.index()],
            EditorAction::Fill | EditorAction::Transform | EditorAction::Adjust => {
                if self.canvas.is_none() {
                    Availability::NothingLeft
                } else if self.drawing {
                    Availability::StrokeInProgress
                } else {
                    Availability::Available
                }
            }
        }
    }

    /// Filling: the threshold, whether a fill is being placed, its notice.
    #[must_use]
    pub const fn fill(&self) -> FillProjection {
        self.fill
    }

    /// The Selection tool, what is selected and the shape being drawn.
    #[must_use]
    pub fn selection(&self) -> &SelectionProjection {
        &self.selection
    }

    /// The Transform tool, its frame and its notice.
    #[must_use]
    pub fn transform(&self) -> TransformProjection {
        *self.transform
    }

    /// The adjustment under way, its values and its notice.
    #[must_use]
    pub fn adjust(&self) -> &AdjustProjection {
        &self.adjust
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

    /// How the artwork shows.
    #[must_use]
    pub const fn view(&self) -> CanvasView {
        self.view
    }

    /// The artwork's size, once an artwork is open.
    #[must_use]
    pub const fn canvas(&self) -> Option<CanvasSize> {
        self.canvas
    }

    /// True while a stroke is being drawn.
    #[must_use]
    pub const fn drawing(&self) -> bool {
        self.drawing
    }

    /// The brush, size and opacity, once the toolbox's projection is added (`with_brush`).
    #[must_use]
    pub const fn brush(&self) -> Option<&BrushProjection> {
        self.brush.as_ref()
    }

    /// This projection with the toolbox's, which the app keeps apart from any one artwork.
    #[must_use]
    pub fn with_brush(self, brush: BrushProjection) -> Self {
        Self {
            brush: Some(brush),
            ..self
        }
    }

    /// Export and the reference image, once the app's projection is added (`with_transfers`).
    #[must_use]
    pub fn transfers(&self) -> Option<&TransferProjection> {
        self.transfers.as_deref()
    }

    /// This projection with Paste offered when `held` (what the app's clipboard holds) was
    /// copied from a canvas of this artwork's size and a layer can be added.
    #[must_use]
    pub fn with_clipboard(mut self, held: Option<CanvasSize>) -> Self {
        let pasting = ClipboardAction::Paste.index();
        self.clipboard[pasting] = paste(held, self.canvas, self.layer_actions[0]);
        self
    }

    /// This projection with the export and reference flows, which belong to the app.
    #[must_use]
    pub fn with_transfers(self, transfers: TransferProjection) -> Self {
        Self {
            transfers: Some(Box::new(transfers)),
            ..self
        }
    }
}

impl Default for SessionProjection {
    fn default() -> Self {
        Self {
            undo: Availability::NothingLeft,
            redo: Availability::NothingLeft,
            retry_save: Availability::NothingLeft,
            dismiss_notice: Availability::NothingLeft,
            layer_actions: [Availability::NothingLeft; 8],
            gallery: Availability::NothingLeft,
            storage: StorageNotice::None,
            layers: None,
            view: CanvasView::FIT,
            canvas: None,
            drawing: false,
            fill: FillProjection {
                threshold: DEFAULT_THRESHOLD,
                active: false,
                pending: false,
                refused: None,
                failed: false,
            },
            selection: Box::default(),
            transform: Box::default(),
            adjust: Box::default(),
            clipboard: [Availability::NothingLeft; 6],
            brush: None,
            transfers: None,
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
