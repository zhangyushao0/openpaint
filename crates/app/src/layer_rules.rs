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
    // A grouped layer at the stack's end can still step out of its group.
    let grouped = artwork
        .layer(layer)
        .is_some_and(|layer| layer.props().group().is_some());
    let possible = match action {
        EditorAction::AddLayer => count < MAX_LAYERS,
        EditorAction::DeleteLayer => count > 1 && position.is_some(),
        EditorAction::RaiseLayer => position.is_some_and(|at| at + 1 < count || grouped),
        EditorAction::LowerLayer => position.is_some_and(|at| at > 0 || grouped),
        EditorAction::EditLayer => position.is_some(),
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
