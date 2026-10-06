//! The Actions panel (Procreate's Actions menu, reduced to what exists): the spanner right of
//! Gallery in the left island opens it as a popover under it. Header "Actions"; under "Add",
//! inserting a picture from the system's picker as a new layer (S26) and Copy & Paste, which
//! opens the copy menu (S40, `copy_bar.rs`); under
//! "Share", saving the artwork as a PNG or JPEG file where the user chooses and sending it to
//! another app; under "Canvas", the Reference switch that shows the floating reference window
//! and the row that chooses its image, and the Drawing Guide (its switch and symmetries, S42);
//! under "Pen", Pen buttons, which shows its page in the
//! panel's place (S41, `pen_buttons_page.rs`). A row that opens a system dialog closes the
//! panel; the switch and the page leave it open. Any contact landing on the canvas outside
//! closes it.

use openpaint_app::{
    Delivery, EditorAction, ExportIntent, ExportScale, Guide, GuideIntent, ImageFormat,
    InsertIntent, Intent, ReferenceIntent, SessionProjection, StrokeIntent, Symmetry,
};

use crate::components::{
    Opener, ScreenArea, choices, hint, icon_toggle, menu_row, popover_left, section, switch_row,
    text_choice, text_choice_wide,
};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::pen_buttons_page::{pen_buttons_page, summary};

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
    /// The scale the export rows write at; 1x until another is chosen, for the app's run.
    scale: ExportScale,
    /// Copy & Paste was tapped: the shell opens the copy menu (S40).
    copy_menu: bool,
    /// The Pen buttons page shows in the panel's place (S41).
    pen_page: bool,
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
        let buttons = projection.brush().map(|brush| brush.buttons);
        let pen_page = self.pen_page;
        let mut page = None;
        let rect = popover_left(ctx, "actions", (opener, self.open), |ui| {
            if let Some(buttons) = buttons.filter(|_| pen_page) {
                if pen_buttons_page(ui, buttons, taps) {
                    page = Some(false);
                }
                return;
            }
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading("Actions");
            });
            close |= self.add_rows(ui, offered(EditorAction::InsertImage), taps);
            section(ui, "Share");
            self.scale_row(ui, projection);
            close |= self.export_rows(ui, offered(EditorAction::Export), taps);
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
            guide_rows(ui, projection.guide(), taps);
            if let Some(buttons) = buttons {
                section(ui, "Pen");
                if menu_row(ui, "Pen buttons", &summary(buttons), true).clicked() {
                    page = Some(true);
                }
            }
        });
        if let Some(pen_page) = page {
            self.pen_page = pen_page;
        }
        if close {
            self.close();
        } else if let Some(rect) = rect {
            self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
        }
    }

    /// The export rows at the scale chosen (offered with `export`). Returns whether one was
    /// tapped, which closes the panel.
    fn export_rows(&self, ui: &mut egui::Ui, export: bool, taps: &mut Vec<Intent>) -> bool {
        let scale = self.scale;
        let mut tapped = false;
        for (label, note, format, delivery) in EXPORTS {
            let fits = scale.fits(format);
            let note = if fits { note } else { "Up to 2×" };
            if menu_row(ui, label, note, fits && export).clicked() {
                let start = ExportIntent::Start {
                    format,
                    delivery,
                    scale,
                };
                push(taps, Intent::Export(start));
                tapped = true;
            }
        }
        tapped
    }

    /// The export scale's words, the one chosen in the ink pill (it slides), then the file's
    /// size in pixels at that scale, quieter.
    fn scale_row(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) {
        ui.horizontal(|ui| {
            ui.set_height(TOUCH_TARGET);
            ui.add_space(SPACE);
            choices(ui, "export scale", |ui, group| {
                for scale in ExportScale::ALL {
                    let chosen = self.scale == scale;
                    if text_choice(ui, group, scale.label(), chosen).clicked() {
                        self.scale = scale;
                    }
                }
            });
            if let Some(canvas) = projection.canvas() {
                let factor = self.scale.factor();
                let size = format!("{} × {}", canvas.width() * factor, canvas.height() * factor);
                hint(ui, &size);
            }
        });
    }

    /// The Add section: Insert image… (offered with `insert`) and Copy & Paste, whose note
    /// teaches the gesture that opens the copy menu too. Returns whether a row closes the panel.
    fn add_rows(&mut self, ui: &mut egui::Ui, insert: bool, taps: &mut Vec<Intent>) -> bool {
        section(ui, "Add");
        let mut close = false;
        if menu_row(ui, "Insert image…", "As a new layer", insert).clicked() {
            push(taps, Intent::Insert(InsertIntent::Choose));
            close = true;
        }
        if menu_row(ui, "Copy & Paste", "Three fingers down", true).clicked() {
            self.copy_menu = true;
            close = true;
        }
        close
    }

    /// Whether Copy & Paste was tapped since the last call.
    pub(crate) fn take_copy_menu(&mut self) -> bool {
        std::mem::take(&mut self.copy_menu)
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.pen_page = false;
        self.area = None;
    }
}

/// The Drawing Guide (S42): its switch, then its symmetries as words in two columns, the one in
/// use in the ink pill while it is on (a word tapped turns it on with that symmetry), and while
/// it is on how to move its centre.
fn guide_rows(ui: &mut egui::Ui, guide: Guide, taps: &mut Vec<Intent>) {
    let intent = |guide| Intent::Stroke(StrokeIntent::Guide(guide));
    if switch_row(ui, "Drawing Guide", guide.on, true).clicked() {
        push(taps, intent(GuideIntent::Show(!guide.on)));
    }
    let half = ((ui.available_width() - 2.0 * SPACE) / 2.0).floor();
    choices(ui, "symmetry", |ui, group| {
        for pair in Symmetry::ALL.chunks(2) {
            ui.horizontal(|ui| {
                ui.add_space(2.0 * SPACE);
                for symmetry in pair {
                    let on = guide.on && guide.symmetry == *symmetry;
                    let word = text_choice_wide(ui, group, symmetry.label(), (true, on), half);
                    if word.clicked() && !on {
                        push(taps, intent(GuideIntent::Symmetry(*symmetry)));
                    }
                }
            });
        }
    });
    if guide.on {
        hint(ui, "A finger moves the centre.");
    }
}

/// egui may run a pass twice; one tap is sent once.
fn push(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}
