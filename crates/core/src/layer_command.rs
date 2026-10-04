//! How each `LayerCommand` turns one artwork version into the next.

use crate::artwork::{Artwork, ArtworkState, MAX_LAYERS};
use crate::command::{Applied, Change, LayerCommand};
use crate::error::CoreError;
use crate::group::prune;
use crate::id::LayerId;
use crate::layer::{Layer, check_opacity};
use crate::layer_moves::{lower, raise, reorder};

pub(crate) fn apply(artwork: &Artwork, command: LayerCommand) -> Result<Applied, CoreError> {
    match command {
        LayerCommand::Create => create(artwork),
        LayerCommand::Delete(id) => delete(artwork, id),
        LayerCommand::Reorder { layer, index } => reorder(artwork, layer, index),
        LayerCommand::Raise(id) => raise(artwork, id),
        LayerCommand::Lower(id) => lower(artwork, id),
        LayerCommand::Select(id) => select(artwork, id),
        LayerCommand::Rename { layer, name } => edit(artwork, layer, |l| l.name = name),
        LayerCommand::SetVisible { layer, visible } => {
            edit(artwork, layer, |l| l.props.visible = visible)
        }
        LayerCommand::SetOpacity { layer, opacity } => {
            let opacity = check_opacity(opacity)?;
            edit(artwork, layer, |l| l.props.opacity = opacity)
        }
        LayerCommand::SetLocked { layer, locked } => {
            edit(artwork, layer, |l| l.props.locked = locked)
        }
        LayerCommand::SetAlphaLocked { layer, locked } => {
            edit(artwork, layer, |l| l.props.alpha_locked = locked)
        }
        LayerCommand::SetClipping { layer, clipping } => {
            edit(artwork, layer, |l| l.props.clipping = clipping)
        }
        LayerCommand::SetBlend { layer, blend } => edit(artwork, layer, |l| l.props.blend = blend),
    }
}

fn create(artwork: &Artwork) -> Result<Applied, CoreError> {
    let state = artwork.state();
    if state.layers.len() >= MAX_LAYERS {
        return Err(CoreError::LayerLimit { max: MAX_LAYERS });
    }
    let below = artwork.position(state.active)?;
    let mut next = state.clone();
    let id = next.ids.take_layer();
    let mut layer = Layer::empty(id);
    layer.props.group = state.layers[below].props.group;
    next.layers.insert(below + 1, layer);
    next.activate(id);
    Ok(content(next))
}

fn delete(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    let index = artwork.position(id)?;
    let state = artwork.state();
    if state.layers.len() == 1 {
        return Err(CoreError::LastLayer);
    }
    let mut next = state.clone();
    next.layers.remove(index);
    prune(&next.layers, &mut next.groups);
    if next.active == id {
        next.activate(next.layers[index.saturating_sub(1)].id);
    }
    Ok(content(next))
}

/// The layer's paint becomes the target (not its mask).
fn select(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    artwork.position(id)?;
    if artwork.active() == id && !artwork.on_mask() {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    next.activate(id);
    Ok(Applied {
        artwork: Artwork::from_state(next),
        change: Change::ActiveLayer,
    })
}

/// Changes one layer's name or properties; reports `Change::None` when nothing differs.
fn edit(
    artwork: &Artwork,
    id: LayerId,
    change: impl FnOnce(&mut Layer),
) -> Result<Applied, CoreError> {
    let index = artwork.position(id)?;
    let mut layer = artwork.state().layers[index].clone();
    change(&mut layer);
    if layer == artwork.state().layers[index] {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    next.layers[index] = layer;
    Ok(content(next))
}

pub(crate) fn content(state: ArtworkState) -> Applied {
    Applied {
        artwork: Artwork::from_state(state),
        change: Change::Content,
    }
}
