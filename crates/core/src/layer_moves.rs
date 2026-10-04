//! Moving layers through the stack, into and out of groups, without ever splitting a group.

use crate::artwork::Artwork;
use crate::command::Applied;
use crate::error::CoreError;
use crate::group::prune;
use crate::id::LayerId;
use crate::layer_command::content;

pub(crate) fn reorder(artwork: &Artwork, id: LayerId, index: usize) -> Result<Applied, CoreError> {
    let from = artwork.position(id)?;
    let len = artwork.state().layers.len();
    if index >= len {
        return Err(CoreError::IndexOutOfRange { index, len });
    }
    if from == index {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    let layer = next.layers.remove(from);
    let own = layer.props.group;
    next.layers.insert(index, layer);
    let below = index
        .checked_sub(1)
        .and_then(|at| next.layers[at].props.group);
    let above = next
        .layers
        .get(index + 1)
        .and_then(|layer| layer.props.group);
    next.layers[index].props.group = match (below, above) {
        (Some(below), Some(above)) if below == above => Some(below),
        _ if own.is_some() && (below == own || above == own) => own,
        _ => None,
    };
    prune(&next.layers, &mut next.groups);
    Ok(content(next))
}

pub(crate) fn raise(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    step(artwork, id, true)
}

pub(crate) fn lower(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    step(artwork, id, false)
}

/// One place up or down: out of the group at its edge, into a group beside it, or past the
/// neighbour.
fn step(artwork: &Artwork, id: LayerId, up: bool) -> Result<Applied, CoreError> {
    let index = artwork.position(id)?;
    let layers = &artwork.state().layers;
    let len = layers.len();
    let neighbour = if up {
        Some(index + 1).filter(|at| *at < len)
    } else {
        index.checked_sub(1)
    };
    let own = layers[index].props.group;
    let beside = neighbour.and_then(|at| layers[at].props.group);
    let mut next = artwork.state().clone();
    if own.is_some() && beside != own {
        // The edge of its group (the stack's end too): it leaves the group where it is.
        next.layers[index].props.group = None;
    } else if let Some(at) = neighbour {
        if own.is_none() && beside.is_some() {
            // Beside a group: it joins it where it is.
            next.layers[index].props.group = beside;
        } else {
            next.layers.swap(index, at);
        }
    } else {
        let index = if up { len } else { 0 };
        return Err(CoreError::IndexOutOfRange { index, len });
    }
    prune(&next.layers, &mut next.groups);
    Ok(content(next))
}
