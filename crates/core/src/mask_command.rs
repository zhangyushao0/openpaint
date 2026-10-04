//! How each `MaskCommand` turns one artwork version into the next.

use crate::artwork::Artwork;
use crate::command::{Applied, Change, MaskCommand};
use crate::error::CoreError;
use crate::id::LayerId;
use crate::layer_command::content;
use crate::mask::LayerMask;

pub(crate) fn apply(artwork: &Artwork, command: MaskCommand) -> Result<Applied, CoreError> {
    match command {
        MaskCommand::Add(id) => add(artwork, id),
        MaskCommand::Select(id) => select(artwork, id),
        MaskCommand::SetEnabled { layer, enabled } => {
            let applied = edit(artwork, layer, false, |mask| mask.enabled = enabled)?;
            Ok(paint_target(applied, layer))
        }
        MaskCommand::Invert(layer) => edit(artwork, layer, true, |mask| {
            mask.inverted = !mask.inverted;
        }),
        MaskCommand::Clear(layer) => edit(artwork, layer, true, |mask| {
            *mask = LayerMask {
                enabled: mask.enabled,
                ..LayerMask::white()
            };
        }),
        MaskCommand::Delete(id) => delete(artwork, id),
    }
}

fn add(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    let index = artwork.editable_layer(id)?;
    if artwork.state().layers[index].mask.is_some() {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    next.layers[index].mask = Some(LayerMask::white());
    next.active = id;
    next.on_mask = true;
    Ok(content(next))
}

/// A disabled mask cannot be the target: disabling the target makes the layer's paint the
/// target, in the same step.
fn paint_target(applied: Applied, id: LayerId) -> Applied {
    let state = applied.artwork.state();
    let off = state.layers.iter().find(|layer| layer.id == id);
    let off = off
        .and_then(|layer| layer.mask.as_ref())
        .is_some_and(|m| !m.enabled);
    if !(off && state.active == id && state.on_mask) {
        return applied;
    }
    let mut next = state.clone();
    next.on_mask = false;
    content(next)
}

fn select(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    let index = artwork.position(id)?;
    match &artwork.state().layers[index].mask {
        None => return Err(CoreError::NoMask(id)),
        Some(mask) if !mask.enabled => return Err(CoreError::MaskOff(id)),
        Some(_) => {}
    }
    if artwork.active() == id && artwork.on_mask() {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    next.active = id;
    next.on_mask = true;
    Ok(Applied {
        artwork: Artwork::from_state(next),
        change: Change::ActiveLayer,
    })
}

/// Changes the layer's mask; `Change::None` when nothing differs. `enabled_only` commands are
/// refused (`MaskOff`) on a disabled mask.
fn edit(
    artwork: &Artwork,
    id: LayerId,
    enabled_only: bool,
    change: impl FnOnce(&mut LayerMask),
) -> Result<Applied, CoreError> {
    let index = artwork.editable_layer(id)?;
    let old = artwork.state().layers[index]
        .mask
        .as_ref()
        .ok_or(CoreError::NoMask(id))?;
    if enabled_only && !old.enabled {
        return Err(CoreError::MaskOff(id));
    }
    let mut mask = old.clone();
    change(&mut mask);
    if mask == *old {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    next.layers[index].mask = Some(mask);
    Ok(content(next))
}

fn delete(artwork: &Artwork, id: LayerId) -> Result<Applied, CoreError> {
    let index = artwork.editable_layer(id)?;
    if artwork.state().layers[index].mask.is_none() {
        return Err(CoreError::NoMask(id));
    }
    let mut next = artwork.state().clone();
    next.layers[index].mask = None;
    if next.active == id {
        next.on_mask = false;
    }
    Ok(content(next))
}
