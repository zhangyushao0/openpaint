//! Adjustments (Procreate's, on a layer): the wand after Actions in the left island (Procreate's
//! place) opens a popover listing them, one `menu_row` each. Choosing one closes it, and the
//! chosen adjustment's panel grows from the same button: its name, its controls
//! (`adjust_controls`) and the commands Cancel and Apply. The canvas shows the preview while
//! the hand sets values; the panel stays while the fingers move the view, and goes when the
//! adjustment is applied or cancelled. The wand carries the ink ring while either shows;
//! tapped while adjusting, it applies, as the Transform button commits.

use openpaint_app::{AdjustIntent, EditorAction, Intent, SessionProjection};
use openpaint_core::{Adjustment, AdjustmentKind, ToneChannel};

use crate::adjust_controls::{controls, heading, push};
use crate::components::{Opener, ScreenArea, icon_toggle, menu_row, popover_left};
use crate::design::Icon;

/// The menu's rows, top to bottom (Procreate's names).
const MENU: [(&str, AdjustmentKind); 4] = [
    (
        "Hue, Saturation, Brightness",
        AdjustmentKind::HueSaturationBrightness,
    ),
    ("Curves", AdjustmentKind::Curves),
    ("Gaussian Blur", AdjustmentKind::GaussianBlur),
    ("Sharpen", AdjustmentKind::Sharpen),
];

#[derive(Debug)]
pub(crate) struct AdjustPanel {
    menu: bool,
    /// Where the open menu or panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the wand lies, in points, from this pass.
    button: Option<egui::Rect>,
    /// The adjustment last shown, drawn while its panel goes back into the wand.
    last: Option<Adjustment>,
    /// The curve the curve editor shows (the panel's own, like Procreate's channel buttons).
    channel: ToneChannel,
}

impl Default for AdjustPanel {
    fn default() -> Self {
        Self {
            menu: false,
            area: None,
            button: None,
            last: None,
            channel: ToneChannel::Composite,
        }
    }
}

impl AdjustPanel {
    /// The wand, in the left island. Returns whether a tap opened the menu.
    pub(crate) fn button(
        &mut self,
        ui: &mut egui::Ui,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) -> bool {
        let adjusting = projection.adjust().adjustment.is_some();
        let response = icon_toggle(ui, Icon::Adjustments, "Adjustments", self.menu || adjusting);
        self.button = Some(response.rect);
        if !response.clicked() {
            return false;
        }
        if adjusting {
            push(taps, Intent::Adjust(AdjustIntent::Apply));
            return false;
        }
        self.menu = !self.menu;
        self.menu
    }

    /// The menu or the adjustment's panel while open or closing, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        let opener = Opener {
            island: "left island",
            button: self.button.unwrap_or(egui::Rect::ZERO),
        };
        let adjustment = projection.adjust().adjustment.clone();
        if adjustment.is_some() {
            self.menu = false;
            self.last.clone_from(&adjustment);
        } else {
            self.channel = ToneChannel::Composite;
        }
        let offered = projection.availability(EditorAction::Adjust).is_offered();
        let mut chosen = false;
        let menu = popover_left(ctx, "adjustments", (opener, self.menu), |ui| {
            heading(ui, "Adjustments");
            for (label, kind) in MENU {
                if menu_row(ui, label, "", offered).clicked() {
                    push(taps, Intent::Adjust(AdjustIntent::Begin(kind)));
                    chosen = true;
                }
            }
        });
        let open = adjustment.is_some();
        let shown = adjustment.or_else(|| self.last.clone());
        let channel = &mut self.channel;
        let panel = popover_left(ctx, "adjustment", (opener, open), |ui| {
            if let Some(shown) = &shown {
                controls(ui, shown, channel, taps);
            }
        });
        if chosen {
            self.menu = false;
        }
        let ppp = ctx.pixels_per_point();
        self.area = menu
            .filter(|_| !chosen)
            .or(panel)
            .map(|rect| ScreenArea::from_points(rect, ppp));
    }

    /// Where the open menu or panel lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    /// Closes the menu (another panel opened, or a contact landed outside); an adjustment's
    /// panel stays until it is applied or cancelled.
    pub(crate) fn close_menu(&mut self) {
        self.menu = false;
    }
}
