//! Which layer actions the panel offers now: the rules the projection reads, so the UI never
//! works them out itself; and the one check an edit of the active layer makes before it starts.

use std::fmt;

use openpaint_core::{Artwork, ArtworkCommand, GroupCommand, LayerId, MAX_LAYERS};

use crate::projection::{Availability, EditorAction};

/// Why the active layer takes no fill, adjustment or transform, for the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerRefusal {
    /// The active layer is hidden, by its eye or its group's: the edit would not show.
    Hidden,
    /// The active layer is locked.
    Locked,
}

impl LayerRefusal {
    /// Why `artwork`'s active layer refuses an edit, if it does.
    #[must_use]
    pub fn of(artwork: &Artwork) -> Option<Self> {
        let layer = artwork.layer(artwork.active())?;
        if !artwork.shown(layer.id()) {
            Some(Self::Hidden)
        } else if layer.props().locked() {
            Some(Self::Locked)
        } else {
            None
        }
    }
}

impl fmt::Display for LayerRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hidden => "the layer is hidden",
            Self::Locked => "the layer is locked",
        })
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
    let unlocked = artwork.layer(layer).filter(|layer| !layer.props().locked());
    let mask = unlocked.and_then(|layer| layer.mask());
    let possible = match action {
        EditorAction::AddMask => unlocked.is_some() && mask.is_none(),
        EditorAction::EditMask => mask.is_some(),
        EditorAction::ChangeMask => mask.is_some_and(|mask| mask.enabled()),
        EditorAction::AddLayer => count < MAX_LAYERS,
        EditorAction::DeleteLayer => count > 1 && position.is_some(),
        EditorAction::EditLayer => position.is_some(),
        EditorAction::MergeDown => artwork.mergeable(layer).is_ok(),
        EditorAction::GroupLayer => {
            let group = ArtworkCommand::Group(GroupCommand::Create { layer });
            position.is_some() && artwork.apply(group).is_ok()
        }
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
