//! The layer intents: what the layer panel asks of the session, and the layer list it shows.
//!
//! Every change of the layers is one history step that undo takes back, with two exceptions
//! (DECISIONS D23): choosing the layer to draw on is not an edit and changes the current step in
//! place, and an opacity slider moved in one touch is one step however many values it passed.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, Change, LayerCommand, LayerId, LayerName, MAX_LAYERS,
};

use crate::error::AppError;
use crate::history::History;
use crate::projection::{Availability, EditorAction};

#[derive(Debug, Clone, PartialEq)]
pub enum LayerIntent {
    /// A new empty layer above the active one, which becomes active.
    Add,
    /// Draw on this layer from now on.
    Select(LayerId),
    /// Remove the layer and its strokes; undo brings both back.
    Delete(LayerId),
    /// One place up, towards the top of the stack.
    Raise(LayerId),
    /// One place down, towards the paper.
    Lower(LayerId),
    SetVisible {
        layer: LayerId,
        visible: bool,
    },
    /// `opacity` in `0..=1`. `settled` is false while the finger is still on the slider: the
    /// values of one touch amend one history step, the settled one ends it.
    SetOpacity {
        layer: LayerId,
        opacity: f32,
        settled: bool,
    },
    /// Alpha Lock on or off: strokes begun while it is on paint only over the layer's paint.
    SetAlphaLock {
        layer: LayerId,
        locked: bool,
    },
}

/// One row of the layer panel.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRow {
    pub id: LayerId,
    pub name: LayerName,
    pub visible: bool,
    /// In `0..=1`.
    pub opacity: f32,
    pub alpha_locked: bool,
}

/// The layers as the panel shows them, bottom to top, and the one drawn on.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerList {
    pub rows: Arc<[LayerRow]>,
    pub active: LayerId,
}

impl LayerList {
    pub(crate) fn of(artwork: &Artwork) -> Self {
        let rows = artwork
            .layers()
            .iter()
            .map(|layer| LayerRow {
                id: layer.id(),
                name: layer.name().clone(),
                visible: layer.props().visible(),
                opacity: layer.props().opacity(),
                alpha_locked: layer.props().alpha_locked(),
            })
            .collect();
        Self {
            rows,
            active: artwork.active(),
        }
    }

    /// The active layer's row.
    #[must_use]
    pub fn active_row(&self) -> Option<&LayerRow> {
        self.rows.iter().find(|row| row.id == self.active)
    }
}

/// Whether `action` can act on `layer` of `artwork` now; `drawing` while a stroke is drawn.
pub(crate) fn availability(
    artwork: &Artwork,
    drawing: bool,
    action: EditorAction,
    layer: LayerId,
) -> Availability {
    let count = artwork.layers().len();
    let position = artwork.layers().iter().position(|l| l.id() == layer);
    let possible = match action {
        EditorAction::AddLayer => count < MAX_LAYERS,
        EditorAction::DeleteLayer => count > 1 && position.is_some(),
        EditorAction::RaiseLayer => position.is_some_and(|at| at + 1 < count),
        EditorAction::LowerLayer => position.is_some_and(|at| at > 0),
        EditorAction::EditLayer => position.is_some(),
        _ => false,
    };
    if !possible && action == EditorAction::AddLayer {
        Availability::Full
    } else if !possible {
        Availability::NothingLeft
    } else if drawing {
        Availability::StrokeInProgress
    } else {
        Availability::Available
    }
}

impl LayerIntent {
    /// The action the projection offers for this intent, and the layer it acts on.
    fn action(&self, active: LayerId) -> (EditorAction, LayerId) {
        match *self {
            Self::Add => (EditorAction::AddLayer, active),
            Self::Delete(layer) => (EditorAction::DeleteLayer, layer),
            Self::Raise(layer) => (EditorAction::RaiseLayer, layer),
            Self::Lower(layer) => (EditorAction::LowerLayer, layer),
            Self::Select(layer)
            | Self::SetVisible { layer, .. }
            | Self::SetOpacity { layer, .. }
            | Self::SetAlphaLock { layer, .. } => (EditorAction::EditLayer, layer),
        }
    }

    /// The core command, given the artwork it applies to.
    fn command(&self, artwork: &Artwork) -> LayerCommand {
        let index = |layer: LayerId| artwork.layers().iter().position(|l| l.id() == layer);
        match *self {
            Self::Add => LayerCommand::Create,
            Self::Select(layer) => LayerCommand::Select(layer),
            Self::Delete(layer) => LayerCommand::Delete(layer),
            Self::Raise(layer) => LayerCommand::Reorder {
                layer,
                index: index(layer).map_or(0, |at| at + 1),
            },
            Self::Lower(layer) => LayerCommand::Reorder {
                layer,
                index: index(layer).map_or(0, |at| at.saturating_sub(1)),
            },
            Self::SetVisible { layer, visible } => LayerCommand::SetVisible { layer, visible },
            Self::SetOpacity { layer, opacity, .. } => LayerCommand::SetOpacity { layer, opacity },
            Self::SetAlphaLock { layer, locked } => LayerCommand::SetAlphaLocked { layer, locked },
        }
    }
}

/// The layer intents' state: the opacity touch whose values amend the current history step.
#[derive(Debug, Default)]
pub(crate) struct Layers {
    sliding: Option<LayerId>,
}

impl Layers {
    /// Any other intent ends an opacity touch, so its next value starts a new step.
    pub(crate) fn interrupt(&mut self) {
        self.sliding = None;
    }

    /// Applies `intent` to the history's current version. `drawing` while a stroke is drawn.
    pub(crate) fn handle(
        &mut self,
        history: &mut History,
        drawing: bool,
        intent: &LayerIntent,
    ) -> Result<(), AppError> {
        let artwork = history.current();
        let (action, layer) = intent.action(artwork.active());
        let availability = availability(artwork, drawing, action, layer);
        if !availability.is_available() {
            self.sliding = None;
            return Err(AppError::Unavailable {
                action,
                availability,
            });
        }
        let command = ArtworkCommand::Layer(intent.command(artwork));
        let applied = artwork.apply(command)?;
        let (sliding, settled) = match *intent {
            LayerIntent::SetOpacity { layer, settled, .. } => (Some(layer), settled),
            _ => (None, true),
        };
        // A touch amends the step its own first change recorded, never an earlier one.
        let continuing = sliding.is_some() && sliding == self.sliding;
        let changed = applied.change != Change::None;
        if applied.change == Change::ActiveLayer || (changed && continuing) {
            history.amend(applied.artwork);
        } else if changed {
            history.record(applied.artwork);
        }
        self.sliding = sliding.filter(|_| !settled && (continuing || changed));
        Ok(())
    }
}
