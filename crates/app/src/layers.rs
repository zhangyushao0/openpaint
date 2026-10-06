//! The layer intents: what the layer panel asks of the session, and the layer list it shows.
//!
//! Every change of the layers, groups and masks is one history step that undo takes back, with
//! two exceptions (DECISIONS D65): choosing the layer to draw on (its paint or its mask) is not
//! an edit and changes the current step in place, and an opacity slider (a layer's or a
//! group's) moved in one touch is one step however many values it passed. Collapsing a group is
//! the panel's own state.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, Change, GroupCommand, GroupId, LayerCommand, LayerId,
    LayerName, LayerPlace, MaskCommand, Unmergeable, layer_places,
};

use crate::error::AppError;
use crate::history::History;
use crate::layer_rules::availability;
use crate::projection::EditorAction;

#[derive(Debug, Clone, PartialEq)]
pub enum LayerIntent {
    /// A new empty layer above the active one, in its group, which becomes active.
    Add,
    /// Draw on this layer from now on.
    Select(LayerId),
    /// Remove the layer and its strokes; undo brings both back.
    Delete(LayerId),
    /// Merge Down (S39): the layer goes into the one below, which becomes active; one step.
    MergeDown(LayerId),
    /// A copy right above the layer, which becomes active (`LayerCommand::Duplicate`).
    Duplicate(LayerId),
    /// Where a dragged row was let go: `by` steps up (above 0) or down, one history step
    /// (`LayerList::places` lists them; `LayerCommand::Move`).
    Move {
        layer: LayerId,
        by: i32,
    },
    Rename {
        layer: LayerId,
        name: LayerName,
    },
    SetVisible {
        layer: LayerId,
        visible: bool,
    },
    /// `opacity` in `0..=1`. `settled` is false while the finger is still on the slider: the
    /// values of one touch amend one history step, the settled one ends it.
    SetOpacity {
        layer: LayerId,
        opacity: f32,
        settled: bool,
    },
    /// Alpha Lock on or off: strokes begun while it is on paint only over the layer's paint.
    SetAlphaLock {
        layer: LayerId,
        locked: bool,
    },
    /// Clipping Mask on or off: the layer shows only on its base's paint.
    SetClipping {
        layer: LayerId,
        clipping: bool,
    },
    SetBlend {
        layer: LayerId,
        blend: BlendMode,
    },
    /// Group the layer with the one under it (its clipping chain stays whole).
    Group(LayerId),
    /// A new empty layer in a new group above the active layer, which becomes active.
    NewGroup,
    /// The group goes; its layers stay where they are.
    Ungroup(GroupId),
    SetGroupVisible {
        group: GroupId,
        visible: bool,
    },
    /// As `SetOpacity`, for a group.
    SetGroupOpacity {
        group: GroupId,
        opacity: f32,
        settled: bool,
    },
    /// A white mask on the layer, which becomes active with its mask the target (S24).
    AddMask(LayerId),
    /// Paint on the layer's mask from now on; like `Select`, not an edit of its own.
    SelectMask(LayerId),
    /// The mask on or off; off, the layer's paint is the target again.
    SetMaskEnabled {
        layer: LayerId,
        enabled: bool,
    },
    InvertMask(LayerId),
    /// Back to white everywhere.
    ClearMask(LayerId),
    DeleteMask(LayerId),
}

/// A layer's mask in the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskRow {
    pub enabled: bool,
    pub inverted: bool,
}

/// One row of the layer panel.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRow {
    pub id: LayerId,
    pub name: LayerName,
    pub visible: bool,
    /// In `0..=1`.
    pub opacity: f32,
    pub alpha_locked: bool,
    /// The Clipping Mask switch.
    pub clipping: bool,
    /// The layer clips to a base under it now (the switch is on and it has a base).
    pub clipped: bool,
    pub blend: BlendMode,
    pub group: Option<GroupId>,
    pub mask: Option<MaskRow>,
}

/// One group of the layer panel.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRow {
    pub id: GroupId,
    pub name: LayerName,
    pub visible: bool,
    /// In `0..=1`.
    pub opacity: f32,
}

/// The layers as the panel shows them, bottom to top, the groups, and the one drawn on.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerList {
    pub rows: Arc<[LayerRow]>,
    pub groups: Arc<[GroupRow]>,
    pub active: LayerId,
    /// Why the active layer cannot merge down now (`None`: it can).
    pub merge_down: Option<Unmergeable>,
    /// Strokes and fills go to the active layer's mask.
    pub on_mask: bool,
}

impl LayerList {
    pub(crate) fn of(artwork: &Artwork) -> Self {
        let rows = artwork
            .layers()
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let props = layer.props();
                LayerRow {
                    id: layer.id(),
                    name: layer.name().clone(),
                    visible: props.visible(),
                    opacity: props.opacity(),
                    alpha_locked: props.alpha_locked(),
                    clipping: props.clipping(),
                    clipped: artwork.clips_at(index),
                    blend: props.blend(),
                    group: props.group(),
                    mask: layer.mask().map(|mask| MaskRow {
                        enabled: mask.enabled(),
                        inverted: mask.inverted(),
                    }),
                }
            })
            .collect();
        let groups = artwork
            .groups()
            .iter()
            .map(|group| GroupRow {
                id: group.id(),
                name: group.name().clone(),
                visible: group.visible(),
                opacity: group.opacity(),
            })
            .collect();
        Self {
            rows,
            groups,
            active: artwork.active(),
            merge_down: artwork.mergeable(artwork.active()).err(),
            on_mask: artwork.on_mask(),
        }
    }

    /// The active layer's row.
    #[must_use]
    pub fn active_row(&self) -> Option<&LayerRow> {
        self.rows.iter().find(|row| row.id == self.active)
    }

    /// Every place a `Move` of `layer` reaches, the farthest down first, its own (`by` 0)
    /// among them (`layer_places`): where its row can be dragged. Empty for a layer not listed.
    #[must_use]
    pub fn places(&self, layer: LayerId) -> Vec<LayerPlace> {
        let groups: Vec<_> = self.rows.iter().map(|row| row.group).collect();
        self.rows
            .iter()
            .position(|row| row.id == layer)
            .map_or_else(Vec::new, |index| layer_places(&groups, index))
    }

    #[must_use]
    pub fn group(&self, id: GroupId) -> Option<&GroupRow> {
        self.groups.iter().find(|group| group.id == id)
    }
}

impl LayerIntent {
    /// The action the projection offers for this intent, and the layer it acts on.
    fn action(&self, active: LayerId) -> (EditorAction, LayerId) {
        match *self {
            Self::Add | Self::NewGroup => (EditorAction::AddLayer, active),
            Self::Duplicate(layer) => (EditorAction::AddLayer, layer),
            Self::MergeDown(layer) => (EditorAction::MergeDown, layer),
            Self::Delete(layer) => (EditorAction::DeleteLayer, layer),
            Self::Group(layer) => (EditorAction::GroupLayer, layer),
            Self::Select(layer)
            | Self::Move { layer, .. }
            | Self::Rename { layer, .. }
            | Self::SetVisible { layer, .. }
            | Self::SetOpacity { layer, .. }
            | Self::SetAlphaLock { layer, .. }
            | Self::SetClipping { layer, .. }
            | Self::SetBlend { layer, .. } => (EditorAction::EditLayer, layer),
            Self::Ungroup(_) | Self::SetGroupVisible { .. } | Self::SetGroupOpacity { .. } => {
                (EditorAction::EditLayer, active)
            }
            Self::AddMask(layer) => (EditorAction::AddMask, layer),
            Self::SetMaskEnabled { layer, .. } | Self::DeleteMask(layer) => {
                (EditorAction::EditMask, layer)
            }
            Self::SelectMask(layer) | Self::InvertMask(layer) | Self::ClearMask(layer) => {
                (EditorAction::ChangeMask, layer)
            }
        }
    }

    /// The core command.
    fn command(&self) -> ArtworkCommand {
        let layer = |command| ArtworkCommand::Layer(command);
        let group = |command| ArtworkCommand::Group(command);
        match *self {
            Self::Add => layer(LayerCommand::Create),
            Self::Select(id) => layer(LayerCommand::Select(id)),
            Self::Delete(id) => layer(LayerCommand::Delete(id)),
            Self::Duplicate(id) => layer(LayerCommand::Duplicate(id)),
            Self::MergeDown(id) => layer(LayerCommand::MergeDown(id)),
            Self::Move { layer: id, by } => layer(LayerCommand::Move { layer: id, by }),
            Self::Rename {
                layer: id,
                ref name,
            } => layer(LayerCommand::Rename {
                layer: id,
                name: name.clone(),
            }),
            Self::SetVisible { layer: id, visible } => {
                layer(LayerCommand::SetVisible { layer: id, visible })
            }
            Self::SetOpacity {
                layer: id, opacity, ..
            } => layer(LayerCommand::SetOpacity { layer: id, opacity }),
            Self::SetAlphaLock { layer: id, locked } => {
                layer(LayerCommand::SetAlphaLocked { layer: id, locked })
            }
            Self::SetClipping {
                layer: id,
                clipping,
            } => layer(LayerCommand::SetClipping {
                layer: id,
                clipping,
            }),
            Self::SetBlend { layer: id, blend } => {
                layer(LayerCommand::SetBlend { layer: id, blend })
            }
            Self::Group(id) => group(GroupCommand::Create { layer: id }),
            Self::NewGroup => group(GroupCommand::New),
            Self::Ungroup(id) => group(GroupCommand::Ungroup(id)),
            Self::SetGroupVisible { group: id, visible } => {
                group(GroupCommand::SetVisible { group: id, visible })
            }
            Self::SetGroupOpacity {
                group: id, opacity, ..
            } => group(GroupCommand::SetOpacity { group: id, opacity }),
            Self::AddMask(id) => ArtworkCommand::Mask(MaskCommand::Add(id)),
            Self::SelectMask(id) => ArtworkCommand::Mask(MaskCommand::Select(id)),
            Self::SetMaskEnabled { layer: id, enabled } => {
                ArtworkCommand::Mask(MaskCommand::SetEnabled { layer: id, enabled })
            }
            Self::InvertMask(id) => ArtworkCommand::Mask(MaskCommand::Invert(id)),
            Self::ClearMask(id) => ArtworkCommand::Mask(MaskCommand::Clear(id)),
            Self::DeleteMask(id) => ArtworkCommand::Mask(MaskCommand::Delete(id)),
        }
    }

    /// The slider this intent moves, and whether its touch ended.
    fn slider(&self) -> Option<(Slider, bool)> {
        match *self {
            Self::SetOpacity { layer, settled, .. } => Some((Slider::Layer(layer), settled)),
            Self::SetGroupOpacity { group, settled, .. } => Some((Slider::Group(group), settled)),
            _ => None,
        }
    }
}

/// What an opacity touch fades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slider {
    Layer(LayerId),
    Group(GroupId),
}

/// The layer intents' state: the opacity touch whose values amend the current history step.
#[derive(Debug, Default)]
pub(crate) struct Layers {
    sliding: Option<Slider>,
}

impl Layers {
    /// Any other intent ends an opacity touch, so its next value starts a new step.
    pub(crate) fn interrupt(&mut self) {
        self.sliding = None;
    }

    /// Applies `intent` to the history's current version. `drawing` while a stroke is drawn.
    pub(crate) fn handle(
        &mut self,
        history: &mut History,
        drawing: bool,
        intent: &LayerIntent,
    ) -> Result<(), AppError> {
        let artwork = history.current();
        let (action, layer) = intent.action(artwork.active());
        let availability = availability(artwork, drawing, action, layer);
        if !availability.is_available() {
            self.sliding = None;
            return Err(AppError::Unavailable {
                action,
                availability,
            });
        }
        let applied = artwork.apply(intent.command())?;
        let (sliding, settled) = intent
            .slider()
            .map_or((None, true), |(slider, settled)| (Some(slider), settled));
        // A touch amends the step its own first change recorded, never an earlier one.
        let continuing = sliding.is_some() && sliding == self.sliding;
        let changed = applied.change != Change::None;
        if applied.change == Change::ActiveLayer || (changed && continuing) {
            history.amend(applied.artwork);
        } else if changed {
            history.record(applied.artwork);
        }
        self.sliding = sliding.filter(|_| !settled && (continuing || changed));
        Ok(())
    }
}
