//! `SessionProjection`: what the UI may offer right now. The UI greys out what there is nothing
//! for (`Availability::is_offered`) and never works out the rules itself. The actions and their
//! availability are `actions.rs`'s.

use openpaint_core::CanvasSize;

use crate::adjust::AdjustProjection;
use crate::clipboard::{ClipboardAction, paste};
use crate::fill::{DEFAULT_THRESHOLD, FillProjection};
use crate::gallery::GalleryProjection;
use crate::layers::LayerList;
use crate::replay::{ReplayProjection, ReplayShown};
use crate::selection::SelectionProjection;
use crate::storage::StorageNotice;
use crate::stroke::ShapeProjection;
use crate::symmetry::Guide;
use crate::tool_intent::BrushProjection;
use crate::transfer::TransferProjection;
use crate::transform::TransformProjection;
use crate::view::CanvasView;

mod actions;

pub use actions::{Availability, EditorAction};

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
    /// The Drawing Guide, and the shape snapped last while it can be edited.
    pub(crate) guide: Guide,
    pub(crate) shape: Option<Box<ShapeProjection>>,
    pub(crate) brush: Option<BrushProjection>,
    /// Boxed: rarely more than a notice, but its errors are large.
    pub(crate) transfers: Option<Box<TransferProjection>>,
    /// The diameter of the pen the mode draws with instead of the brush (canvas pixels), for the
    /// hover outline (`with_pen`).
    pub(crate) pen: Option<f32>,
    /// The replay: Actions' row as the session offers it, none on a mode's sheet, or its screen
    /// while it is open (`with_replay`, S163).
    pub(crate) replay: ReplayShown,
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
            EditorAction::Replay => match &self.replay {
                ReplayShown::Offered { availability, .. } => *availability,
                ReplayShown::None | ReplayShown::Open(_) => Availability::NothingLeft,
            },
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

    /// The Drawing Guide: on or off, its symmetry and centre.
    #[must_use]
    pub const fn guide(&self) -> Guide {
        self.guide
    }

    /// The shape the last stroke snapped to, while it can be edited.
    #[must_use]
    pub fn shape(&self) -> Option<&ShapeProjection> {
        self.shape.as_deref()
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

    /// The diameter of the pen the mode draws with instead of the brush (canvas pixels), while it
    /// draws with its own.
    #[must_use]
    pub const fn pen(&self) -> Option<f32> {
        self.pen
    }

    /// This projection with the diameter of the pen a mode draws with instead of the brush, which
    /// the composition root asks of the mode.
    #[must_use]
    pub fn with_pen(self, pen: Option<f32>) -> Self {
        Self { pen, ..self }
    }

    /// This projection with the export and reference flows, which belong to the app.
    #[must_use]
    pub fn with_transfers(self, transfers: TransferProjection) -> Self {
        Self {
            transfers: Some(Box::new(transfers)),
            ..self
        }
    }

    /// The replay as Actions offers it (`Offered`), or its screen while it is open: the canvas then
    /// shows the replay's frame and its chrome only (S163).
    #[must_use]
    pub const fn replay_shown(&self) -> &ReplayShown {
        &self.replay
    }

    /// The replay's screen, while it is open.
    #[must_use]
    pub fn replay(&self) -> Option<&ReplayProjection> {
        match &self.replay {
            ReplayShown::Open(replay) => Some(replay),
            ReplayShown::None | ReplayShown::Offered { .. } => None,
        }
    }

    /// This projection with the replay as the composition root holds it: `None` on a mode's sheet,
    /// `Open` while the replay of the open artwork shows; the session's own offer otherwise.
    #[must_use]
    pub fn with_replay(self, replay: ReplayShown) -> Self {
        Self { replay, ..self }
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
            guide: Guide::default(),
            shape: None,
            brush: None,
            transfers: None,
            pen: None,
            replay: ReplayShown::None,
        }
    }
}
