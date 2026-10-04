//! The Actions panel (Procreate's Actions menu, reduced to what exists): the spanner right of
//! Gallery in the left island opens it as a popover under it. Header "Actions"; under "Add",
//! inserting a picture from the system's picker as a new layer (S26); under
//! "Share", saving the artwork as a PNG or JPEG file where the user chooses and sending it to
//! another app; under "Canvas", the Reference switch that shows the floating reference window
//! and the row that chooses its image. A row that opens a system dialog closes the panel; the
//! switch leaves it open. Any contact landing on the canvas outside closes it.

use openpaint_app::{
    Delivery, EditorAction, ExportIntent, ImageFormat, InsertIntent, Intent, ReferenceIntent,
    SessionProjection,
};

use crate::components::{
    Opener, ScreenArea, icon_toggle, menu_row, popover_left, section, switch_row,
};
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
    /// Where the Actions button lies, in points, from this pass.
    button: Option<egui::Rect>,
}

impl ActionsPanel {
    /// The Actions button, in the left island. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui) -> bool {
        let response = icon_toggle(ui, Icon::Actions, "Actions", self.open);
        self.button = Some(response.rect);
        if response.clicked() {
            self.open = !self.open;
            return self.open;
        }
        false
    }

    /// Where the Actions button lies, in points: the reference window comes out of it.
    pub(crate) fn button_rect(&self) -> egui::Rect {
        self.button.unwrap_or(egui::Rect::ZERO)
    }

    /// The panel while it is open or closing, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.area = None;
        let Some(transfers) = projection.transfers() else {
            return;
        };
        let offered = |action| projection.availability(action).is_available();
        let mut close = false;
        let opener = Opener {
            island: "left island",
            button: self.button_rect(),
        };
        let rect = popover_left(ctx, "actions", (opener, self.open), |ui| {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading("Actions");
            });
            section(ui, "Add");
            let insert = offered(EditorAction::InsertImage);
            if menu_row(ui, "Insert image…", "As a new layer", insert).clicked() {
                push(taps, Intent::Insert(InsertIntent::Choose));
                close = true;
            }
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
        } else if let Some(rect) = rect {
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
