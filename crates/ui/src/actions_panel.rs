//! The Actions panel (Procreate's Actions menu, reduced to what exists): the spanner right of
//! Gallery in the left island opens it as a popover under it. Header "Actions"; under "Add",
//! inserting a picture from the system's picker as a new layer and Copy & Paste, which
//! opens the copy menu (`copy_bar.rs`); under
//! "Share", saving the artwork as a PNG or JPEG file where the user chooses and sending it to
//! another app; under "Canvas", the Reference switch that shows the floating reference window
//! and the row that chooses its image, and the Drawing Guide (its switch and symmetries);
//! under "Pen", Pen buttons, which shows its page in the panel's place
//! (`pen_buttons_page.rs`); then Language with English and 中文, each in its own words, the one
//! shown in the ink pill (Procreate's Prefs; D100); last, quietly, the app's version and
//! storage format ("OpenPaint 1.3.0 · format 1"). A row that opens a system dialog closes the
//! panel; the switch, the words and the page leave it open. Any contact landing on the canvas
//! outside closes it.

use openpaint_app::{
    Delivery, EditorAction, ExportIntent, ExportScale, Guide, GuideIntent, ImageFormat,
    InsertIntent, Intent, Language, ReferenceIntent, SessionProjection, StrokeIntent, Symmetry,
    ToolIntent,
};

use crate::components::{
    Lead, Opener, ScreenArea, choices, hint, icon_toggle, menu_row, panel_heading, popover_left,
    section, switch_row, text_choice, word_columns,
};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::pen_buttons_page::{pen_buttons_page, summary};
use crate::taps::push_once;
use crate::text::{self as t, Text, Words, language};

/// The export rows, top to bottom.
const EXPORTS: [(Text, Option<Text>, ImageFormat, Delivery); 4] = [
    (
        t::SAVE_PNG,
        Some(t::LOSSLESS),
        ImageFormat::Png,
        Delivery::Save,
    ),
    (
        t::SAVE_JPEG,
        Some(t::SMALLER),
        ImageFormat::Jpeg,
        Delivery::Save,
    ),
    (t::SHARE_PNG, None, ImageFormat::Png, Delivery::Share),
    (t::SHARE_JPEG, None, ImageFormat::Jpeg, Delivery::Share),
];

/// The width of the Language row's name, left of its two words, in points.
const LANGUAGE_LABEL: f32 = 12.0 * SPACE;

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
    /// Copy & Paste was tapped: the shell opens the copy menu.
    copy_menu: bool,
    /// The Pen buttons page shows in the panel's place.
    pen_page: bool,
    /// The app's version and storage format, the panel's quiet last line.
    pub(crate) version: Option<(String, u32)>,
}

impl ActionsPanel {
    /// The Actions button, in the left island. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui) -> bool {
        let response = icon_toggle(ui, Icon::Actions, t::ACTIONS, self.open);
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
            panel_heading(ui, t::ACTIONS);
            close |= self.add_rows(ui, offered(EditorAction::InsertImage), taps);
            section(ui, t::SHARE);
            self.scale_row(ui, projection);
            close |= self.export_rows(ui, offered(EditorAction::Export), taps);
            section(ui, t::CANVAS_SECTION);
            let shown = transfers.reference_shown;
            if switch_row(ui, t::REFERENCE, shown, true).clicked() {
                push_once(taps, Intent::Reference(ReferenceIntent::Show(!shown)));
            }
            let choose = offered(EditorAction::ChooseReference);
            if menu_row(ui, t::CHOOSE_REFERENCE, Words::EMPTY, choose).clicked() {
                push_once(taps, Intent::Reference(ReferenceIntent::Choose));
                close = true;
            }
            guide_rows(ui, projection.guide(), taps);
            if let Some(buttons) = buttons {
                section(ui, t::PEN);
                let summary = summary(language(ui.ctx()), buttons);
                if menu_row(ui, t::PEN_BUTTONS, Words::made(&summary), true).clicked() {
                    page = Some(true);
                }
            }
            language_rows(ui, taps);
            if let Some((version, format)) = &self.version {
                let line = t::version_line(language(ui.ctx()), version, *format);
                hint(ui, Words::made(&line));
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
            let note = if fits { note } else { Some(t::UP_TO_2X) };
            let note = note.map_or(Words::EMPTY, Words::from);
            if menu_row(ui, label, note, fits && export).clicked() {
                let start = ExportIntent::Start {
                    format,
                    delivery,
                    scale,
                };
                push_once(taps, Intent::Export(start));
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
                    let words = Words::made(scale.label());
                    if text_choice(ui, group, words, chosen).clicked() {
                        self.scale = scale;
                    }
                }
            });
            if let Some(canvas) = projection.canvas() {
                let factor = self.scale.factor();
                let size = format!("{} × {}", canvas.width() * factor, canvas.height() * factor);
                hint(ui, Words::made(&size));
            }
        });
    }

    /// The Add section: Insert image… (offered with `insert`) and Copy & Paste, whose note
    /// teaches the gesture that opens the copy menu too. Returns whether a row closes the panel.
    fn add_rows(&mut self, ui: &mut egui::Ui, insert: bool, taps: &mut Vec<Intent>) -> bool {
        section(ui, t::ADD);
        let mut close = false;
        if menu_row(ui, t::INSERT_IMAGE, t::AS_A_NEW_LAYER, insert).clicked() {
            push_once(taps, Intent::Insert(InsertIntent::Choose));
            close = true;
        }
        if menu_row(ui, t::COPY_AND_PASTE, t::THREE_FINGERS_DOWN, true).clicked() {
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

/// The Drawing Guide: its switch, then its symmetries as words in two columns, the one in use in
/// the ink pill while it is on (a word tapped turns it on with that symmetry), and while it is on
/// how to move its centre.
fn guide_rows(ui: &mut egui::Ui, guide: Guide, taps: &mut Vec<Intent>) {
    let intent = |guide| Intent::Stroke(StrokeIntent::Guide(guide));
    if switch_row(ui, t::DRAWING_GUIDE, guide.on, true).clicked() {
        push_once(taps, intent(GuideIntent::Show(!guide.on)));
    }
    let words = Symmetry::ALL.map(|symmetry| {
        let on = guide.on && guide.symmetry == symmetry;
        (Words::from(t::symmetry(symmetry)), on, true)
    });
    if let Some(at) = word_columns(ui, "symmetry", &words, Lead::Indent) {
        push_once(taps, intent(GuideIntent::Symmetry(Symmetry::ALL[at])));
    }
    if guide.on {
        hint(ui, t::MOVE_THE_CENTRE);
    }
}

/// The interface's language on one row after its name (the panel stays within a landscape
/// screen with the Drawing Guide's hint): each in its own words, the one shown in the ink
/// pill; a tap switches the whole interface at once, and the toolbox keeps it.
fn language_rows(ui: &mut egui::Ui, taps: &mut Vec<Intent>) {
    let shown = language(ui.ctx());
    let words =
        Language::ALL.map(|each| (Words::from(t::language_name(each)), each == shown, true));
    let lead = Lead::Label(t::LANGUAGE.into(), LANGUAGE_LABEL);
    if let Some(at) = word_columns(ui, "language", &words, lead) {
        push_once(taps, Intent::Tool(ToolIntent::Language(Language::ALL[at])));
    }
}

#[cfg(test)]
#[path = "actions_panel_tests.rs"]
mod tests;
