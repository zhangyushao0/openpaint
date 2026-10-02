//! How each `LayerCommand` turns one artwork version into the next.

use crate::artwork::{Artwork, ArtworkState, MAX_LAYERS};
use crate::command::{Applied, Change, LayerCommand};
use crate::error::CoreError;
use crate::id::LayerId;
use crate::layer::{Layer, check_opacity};

pub(crate) fn apply(artwork: &Artwork, command: LayerCommand) -> Result<Applied, CoreError> {
    match command {
        LayerCommand::Create => create(artwork),
        LayerCommand::Delete(id) => delete(artwork, id),
        LayerCommand::Reorder { layer, index } => reorder(artwork, layer, index),
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
    }
}

fn create(artwork: &Artwork) -> Result<Applied, CoreError> {
    let state = artwork.state();
    if state.layers.len() >= MAX_LAYERS {
        return Err(CoreError::LayerLimit { max: MAX_LAYERS });
    }
    let above = artwork.position(state.active)? + 1;
    let mut next = state.clone();
    let id = next.ids.take_layer();
    next.layers.insert(above, Layer::empty(id));
    next.active = id;
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
    if next.active == id {
        next.active = next.layers[index.saturating_sub(1)].id;
    }
    Ok(content(next))
}

fn reorder(artwork: &Artwork, id: LayerId, index: usize) -> Result<Applied, CoreError> {
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
    next.layers.insert(index, layer);
    Ok(content(next))
}

fn select(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    artwork.position(id)?;
    if artwork.active() == id {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    next.active = id;
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

fn content(state: ArtworkState) -> Applied {
    Applied {
        artwork: Artwork::from_state(state),
        change: Change::Content,
    }
}
