//! `LayerGroup`: a named run of neighbouring layers shown, hidden and faded together. Groups are
//! one level deep: a group holds layers, never groups. A group always holds at least one layer
//! and its layers lie next to each other in the stack; it is removed with its last layer.

use crate::error::CoreError;
use crate::id::GroupId;
use crate::layer::{Layer, check_opacity};
use crate::value::{LayerName, invalid};

/// A group's own properties; its layers name it in `LayerProps::group`.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerGroup {
    pub(crate) id: GroupId,
    pub(crate) name: LayerName,
    pub(crate) visible: bool,
    pub(crate) opacity: f32,
}

impl LayerGroup {
    /// A shown, fully opaque group named after its id ("Group 7").
    pub(crate) fn numbered(id: GroupId) -> Self {
        Self {
            id,
            name: LayerName::titled("Group", id.get()),
            visible: true,
            opacity: 1.0,
        }
    }

    pub(crate) fn new(
        id: GroupId,
        name: LayerName,
        visible: bool,
        opacity: f32,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            id,
            name,
            visible,
            opacity: check_opacity(opacity)?,
        })
    }

    #[must_use]
    pub const fn id(&self) -> GroupId {
        self.id
    }

    #[must_use]
    pub const fn name(&self) -> &LayerName {
        &self.name
    }

    /// A hidden group hides all its layers, whatever their own eyes say.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    /// In `0..=1`: what shows of the group's result over what lies under it.
    #[must_use]
    pub const fn opacity(&self) -> f32 {
        self.opacity
    }
}

/// `layers` (bottom to top) and `groups` keep the group rules: every layer's group exists, every
/// group holds at least one layer, and each group's layers lie next to each other.
pub(crate) fn check_groups(layers: &[Layer], groups: &[LayerGroup]) -> Result<(), CoreError> {
    let mut seen: Vec<GroupId> = Vec::new();
    let mut previous = None;
    for layer in layers {
        let group = layer.props.group;
        if group != previous
            && let Some(id) = group
        {
            if seen.contains(&id) || !groups.iter().any(|g| g.id == id) {
                return Err(invalid("layer groups"));
            }
            seen.push(id);
        }
        previous = group;
    }
    if seen.len() == groups.len() {
        Ok(())
    } else {
        Err(invalid("layer groups"))
    }
}

/// The positions (bottom to top) of the layers of `group`; empty when it holds none.
pub(crate) fn members(layers: &[Layer], group: GroupId) -> std::ops::Range<usize> {
    let start = layers
        .iter()
        .position(|layer| layer.props.group == Some(group));
    let Some(start) = start else {
        return 0..0;
    };
    let count = layers[start..]
        .iter()
        .take_while(|layer| layer.props.group == Some(group))
        .count();
    start..start + count
}

/// `groups` without those no layer names any more.
pub(crate) fn prune(layers: &[Layer], groups: &mut Vec<LayerGroup>) {
    groups.retain(|group| layers.iter().any(|l| l.props.group == Some(group.id)));
}
