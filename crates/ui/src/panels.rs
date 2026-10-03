//! The canvas's popovers, one open at a time: Actions under the left island; brushes, layers
//! and colour under the right one. Opening one closes the others, which then retract into
//! their own buttons while the tapped one grows out of its button (`components::popover`).

use openpaint_app::{Intent, SessionProjection};

use crate::actions_panel::ActionsPanel;
use crate::brush_panel::BrushPanel;
use crate::color_panel::ColorPanel;
use crate::components::ScreenArea;
use crate::layer_panel::LayerPanel;

/// One of the popovers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Actions,
    Layers,
    Brushes,
    Colors,
}

#[derive(Debug, Default)]
pub(crate) struct Panels {
    pub(crate) actions: ActionsPanel,
    pub(crate) layers: LayerPanel,
    pub(crate) brushes: BrushPanel,
    pub(crate) colors: ColorPanel,
}

impl Panels {
    /// Every panel open or still closing, under the islands.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.actions.show(ctx, projection, taps);
        self.layers.show(ctx, projection, taps);
        self.brushes.show(ctx, projection, taps);
        self.colors.show(ctx, projection, taps);
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) fn area(&self) -> Option<ScreenArea> {
        self.actions
            .area()
            .or(self.layers.area())
            .or(self.brushes.area())
            .or(self.colors.area())
    }

    pub(crate) fn close(&mut self) {
        self.actions.close();
        self.layers.close();
        self.brushes.close();
        self.colors.close();
    }

    /// Closes every panel but `open`.
    pub(crate) fn close_but(&mut self, open: Panel) {
        if open != Panel::Actions {
            self.actions.close();
        }
        if open != Panel::Layers {
            self.layers.close();
        }
        if open != Panel::Brushes {
            self.brushes.close();
        }
        if open != Panel::Colors {
            self.colors.close();
        }
    }
}
