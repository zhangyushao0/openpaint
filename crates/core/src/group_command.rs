//! How each `GroupCommand` turns one artwork version into the next.

use crate::artwork::{Artwork, MAX_LAYERS};
use crate::command::{Applied, GroupCommand};
use crate::error::CoreError;
use crate::group::{LayerGroup, members};
use crate::id::{GroupId, LayerId};
use crate::layer::{Layer, check_opacity};
use crate::layer_command::content;

pub(crate) fn apply(artwork: &Artwork, command: GroupCommand) -> Result<Applied, CoreError> {
    match command {
        GroupCommand::Create { layer } => create(artwork, layer),
        GroupCommand::New => new(artwork),
        GroupCommand::Ungroup(group) => ungroup(artwork, group),
        GroupCommand::SetVisible { group, visible } => {
            edit(artwork, group, |g| g.visible = visible)
        }
        GroupCommand::SetOpacity { group, opacity } => {
            let opacity = check_opacity(opacity)?;
            edit(artwork, group, |g| g.opacity = opacity)
        }
    }
}

/// The positions `GroupCommand::Create` groups for the layer at `index`.
fn grouped_range(artwork: &Artwork, index: usize) -> Result<std::ops::Range<usize>, CoreError> {
    let layers = artwork.layers();
    if layers[index].props.group.is_some() {
        return Err(CoreError::GroupLayered(layers[index].id));
    }
    let chain = artwork.chain(index);
    if chain.len() > 1 || chain.start == 0 {
        return Ok(chain);
    }
    let below = &layers[chain.start - 1];
    if below.props.group.is_some() {
        return Err(CoreError::GroupLayered(below.id));
    }
    Ok(artwork.chain(chain.start - 1).start..chain.end)
}

fn create(artwork: &Artwork, layer: LayerId) -> Result<Applied, CoreError> {
    let index = artwork.position(layer)?;
    let range = grouped_range(artwork, index)?;
    let mut next = artwork.state().clone();
    let id = next.ids.take_group();
    for layer in &mut next.layers[range] {
        layer.props.group = Some(id);
    }
    next.groups.push(LayerGroup::numbered(id));
    Ok(content(next))
}

fn new(artwork: &Artwork) -> Result<Applied, CoreError> {
    let state = artwork.state();
    if state.layers.len() >= MAX_LAYERS {
        return Err(CoreError::LayerLimit { max: MAX_LAYERS });
    }
    let index = artwork.position(state.active)?;
    let at = match state.layers[index].props.group {
        Some(group) => members(&state.layers, group).end,
        None => artwork.chain(index).end,
    };
    let mut next = state.clone();
    let layer_id = next.ids.take_layer();
    let group = next.ids.take_group();
    let mut layer = Layer::empty(layer_id);
    layer.props.group = Some(group);
    next.layers.insert(at, layer);
    next.groups.push(LayerGroup::numbered(group));
    next.active = layer_id;
    Ok(content(next))
}

fn ungroup(artwork: &Artwork, group: GroupId) -> Result<Applied, CoreError> {
    let shown = artwork
        .group(group)
        .ok_or(CoreError::UnknownGroup(group))?
        .visible;
    let mut next = artwork.state().clone();
    for layer in &mut next.layers[members(&artwork.state().layers, group)] {
        layer.props.group = None;
        layer.props.visible &= shown;
    }
    next.groups.retain(|g| g.id != group);
    Ok(content(next))
}

/// Changes one group's properties; reports `Change::None` when nothing differs.
fn edit(
    artwork: &Artwork,
    id: GroupId,
    change: impl FnOnce(&mut LayerGroup),
) -> Result<Applied, CoreError> {
    let index = artwork
        .groups()
        .iter()
        .position(|group| group.id == id)
        .ok_or(CoreError::UnknownGroup(id))?;
    let mut group = artwork.groups()[index].clone();
    change(&mut group);
    if group == artwork.groups()[index] {
        return Ok(Applied::unchanged(artwork));
    }
    let mut next = artwork.state().clone();
    next.groups[index] = group;
    Ok(content(next))
}
