//! `Layer` and its properties. Layers change only through artwork commands and stroke sessions.

use crate::error::CoreError;
use crate::id::LayerId;
use crate::stroke_list::StrokeList;
use crate::value::{LayerName, invalid, is_unit};

/// Layer properties the user toggles or slides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerProps {
    pub(crate) visible: bool,
    pub(crate) opacity: f32,
    pub(crate) locked: bool,
}

impl LayerProps {
    pub(crate) fn new(visible: bool, opacity: f32, locked: bool) -> Result<Self, CoreError> {
        Ok(Self {
            visible,
            opacity: check_opacity(opacity)?,
            locked,
        })
    }

    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    /// In `0..=1`.
    #[must_use]
    pub const fn opacity(&self) -> f32 {
        self.opacity
    }

    /// A locked layer's content cannot change; its properties and position still can.
    #[must_use]
    pub const fn locked(&self) -> bool {
        self.locked
    }
}

impl Default for LayerProps {
    /// Visible, fully opaque, unlocked.
    fn default() -> Self {
        Self {
            visible: true,
            opacity: 1.0,
            locked: false,
        }
    }
}

pub(crate) fn check_opacity(opacity: f32) -> Result<f32, CoreError> {
    if is_unit(opacity) {
        Ok(opacity)
    } else {
        Err(invalid("layer opacity"))
    }
}

/// One layer: identity, name, properties and strokes. Clone is cheap (shared strokes and name).
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub(crate) id: LayerId,
    pub(crate) name: LayerName,
    pub(crate) props: LayerProps,
    pub(crate) strokes: StrokeList,
}

impl Layer {
    /// An empty layer with default properties.
    pub(crate) fn empty(id: LayerId) -> Self {
        Self {
            id,
            name: LayerName::numbered(id.get()),
            props: LayerProps::default(),
            strokes: StrokeList::default(),
        }
    }

    #[must_use]
    pub const fn id(&self) -> LayerId {
        self.id
    }

    #[must_use]
    pub const fn name(&self) -> &LayerName {
        &self.name
    }

    #[must_use]
    pub const fn props(&self) -> &LayerProps {
        &self.props
    }

    /// Oldest first.
    #[must_use]
    pub const fn strokes(&self) -> &StrokeList {
        &self.strokes
    }
}
