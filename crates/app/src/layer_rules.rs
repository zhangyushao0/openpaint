//! Which layer actions the panel offers now: the rules the projection reads, so the UI never
//! works them out itself.

use openpaint_core::{Artwork, ArtworkCommand, GroupCommand, LayerId, MAX_LAYERS};

use crate::projection::{Availability, EditorAction};

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
