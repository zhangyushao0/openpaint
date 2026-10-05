//! Moving layers through the stack, into and out of groups, without ever splitting a group. One
//! step's rule (`stepped`) reads only which group each layer lies in, so the places a dragged
//! row can take (`layer_places`) follow from the same rule as the commands.

use crate::artwork::Artwork;
use crate::command::Applied;
use crate::error::CoreError;
use crate::group::prune;
use crate::id::{GroupId, LayerId};
use crate::layer_command::content;

/// Where a layer lies after `by` steps of `Raise` (above 0) or `Lower` (below 0): its index
/// from the bottom of the stack and its group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerPlace {
    pub by: i32,
    pub index: usize,
    pub group: Option<GroupId>,
}

/// Every place the layer at `index` reaches step by step, over a stack whose layers lie in
/// `groups` (bottom first), from the farthest down to the farthest up, its own (`by` 0) among
/// them. A layer passes each group edge in two steps: out of (or into) the group, then on.
#[must_use]
pub fn layer_places(groups: &[Option<GroupId>], index: usize) -> Vec<LayerPlace> {
    let Some(&group) = groups.get(index) else {
        return Vec::new();
    };
    let here = LayerPlace {
        by: 0,
        index,
        group,
    };
    let mut down = walk(groups, here, false);
    down.reverse();
    down.push(here);
    down.extend(walk(groups, here, true));
    down
}

/// The places after one step, two, ... in one direction until the stack's end.
fn walk(groups: &[Option<GroupId>], from: LayerPlace, up: bool) -> Vec<LayerPlace> {
    let mut groups = groups.to_vec();
    let mut place = from;
    let mut places = Vec::new();
    while let Some((index, group)) = stepped(&groups, place.index, up) {
        groups.remove(place.index);
        groups.insert(index, group);
        place = LayerPlace {
            by: place.by + if up { 1 } else { -1 },
            index,
            group,
        };
        places.push(place);
    }
    places
}

/// One place up or down for the layer at `index`: out of its group at the group's edge (it
/// stays where it is), into a group beside it (where it is), or past the neighbour; `None` at
/// the stack's end outside a group. The new index and group.
fn stepped(groups: &[Option<GroupId>], index: usize, up: bool) -> Option<(usize, Option<GroupId>)> {
    let neighbour = if up {
        Some(index + 1).filter(|at| *at < groups.len())
    } else {
        index.checked_sub(1)
    };
    let own = groups[index];
    let beside = neighbour.and_then(|at| groups[at]);
    if own.is_some() && beside != own {
        return Some((index, None));
    }
    let at = neighbour?;
    if own.is_none() && beside.is_some() {
        Some((index, beside))
    } else {
        Some((at, own))
    }
}

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

/// `by` single steps (`stepped`) as one change; 0 changes nothing.
pub(crate) fn move_by(artwork: &Artwork, id: LayerId, by: i32) -> Result<Applied, CoreError> {
    let from = artwork.position(id)?;
    if by == 0 {
        return Ok(Applied::unchanged(artwork));
    }
    let layers = &artwork.state().layers;
    let len = layers.len();
    let mut groups: Vec<Option<GroupId>> = layers.iter().map(|l| l.props.group).collect();
    let (mut index, mut group) = (from, groups[from]);
    for _ in 0..by.unsigned_abs() {
        let Some(next) = stepped(&groups, index, by > 0) else {
            let index = if by > 0 { len } else { 0 };
            return Err(CoreError::IndexOutOfRange { index, len });
        };
        groups.remove(index);
        (index, group) = next;
        groups.insert(index, group);
    }
    let mut next = artwork.state().clone();
    let mut layer = next.layers.remove(from);
    layer.props.group = group;
    next.layers.insert(index, layer);
    prune(&next.layers, &mut next.groups);
    Ok(content(next))
}
