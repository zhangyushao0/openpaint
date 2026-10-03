//! The Actions panel (Procreate's Actions menu, reduced to what exists): the spanner right of
//! Gallery in the left island opens it as a popover under it. Header "Actions"; under
//! "Share", saving the artwork as a PNG or JPEG file where the user chooses and sending it to
//! another app; under "Canvas", the Reference switch that shows the floating reference window
//! and the row that chooses its image. A row that opens a system dialog closes the panel; the
//! switch leaves it open. Any contact landing on the canvas outside closes it.

use openpaint_app::{
    Delivery, EditorAction, ExportIntent, ImageFormat, Intent, ReferenceIntent, SessionProjection,
};

use crate::components::{ScreenArea, icon_toggle, menu_row, popover_left, section, switch_row};
use crate::design::{Icon, SPACE, TOUCH_TARGET};

/// The export rows, top to bottom.
const EXPORTS: [(&str, &str, ImageFormat, Delivery); 4] = [
    ("Save PNG…", "Lossless", ImageFormat::Png, Delivery::Save),
    ("Save JPEG…", "Smaller", ImageFormat::Jpeg, Delivery::Save),
    ("Share PNG…", "", ImageFormat::Png, Delivery::Share),
    ("Share JPEG…", "", ImageFormat::Jpeg, Delivery::Share),
];

/// The panel's UI-local state.
#[derive(Debug, Default)]
pub(crate) struct ActionsPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
}

impl ActionsPanel {
    /// The Actions button, in the left island. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui) -> bool {
        if icon_toggle(ui, Icon::Actions, "Actions", self.open).clicked() {
            self.open = !self.open;
            return self.open;
        }
        false
    }

    /// The panel when it is open, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        let transfers = projection.transfers().filter(|_| self.open);
        let Some(transfers) = transfers else {
            self.area = None;
            return;
        };
        let offered = |action| projection.availability(action).is_available();
        let mut close = false;
        let rect = popover_left(ctx, "actions", |ui| {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading("Actions");
            });
            section(ui, "Share");
            for (label, note, format, delivery) in EXPORTS {
                if menu_row(ui, label, note, offered(EditorAction::Export)).clicked() {
                    push(
                        taps,
                        Intent::Export(ExportIntent::Start { format, delivery }),
                    );
                    close = true;
                }
            }
            section(ui, "Canvas");
            let shown = transfers.reference_shown;
            if switch_row(ui, "Reference", shown, true).clicked() {
                push(taps, Intent::Reference(ReferenceIntent::Show(!shown)));
            }
            let choose = offered(EditorAction::ChooseReference);
            if menu_row(ui, "Choose reference image…", "", choose).clicked() {
                push(taps, Intent::Reference(ReferenceIntent::Choose));
                close = true;
            }
        });
        if close {
            self.close();
        } else {
            self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
        }
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
    }
}

/// egui may run a pass twice; one tap is sent once.
fn push(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}
