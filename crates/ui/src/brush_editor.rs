//! The brush editor (S46, Procreate's Brush Studio reduced to the engine's settings): tapping the
//! chosen brush's row again turns the brush panel into it in place, as Actions turns into the
//! Pen buttons page. At the top the brush's name (tapped: the Rename dialog) and Back; under it
//! the brush's set (S47: Sketching, Inking, Painting, Texture as words, its own in the ink pill;
//! another tapped moves it to that set's end); then the preview strip (`brush_strip.rs`); then
//! the settings in one scrolling column (`brush_settings.rs`), or a dynamics page's curves
//! (`brush_dynamics.rs`, Back to the settings); at the bottom Duplicate, and Reset for a
//! built-in (grey while it is as its file has it) or Delete for a copy (asked first: it cannot
//! be undone). Every change applies to the next stroke of every tool that draws with the brush;
//! strokes drawn keep their look; none is an undo step, as colours are not.

use egui::{Align, Align2, Layout, Rect, Sense, TextStyle, vec2};
use openpaint_app::{
    BrushIntent, BrushProjection, BrushSet, Dynamic, Intent, MAX_BRUSH_NAME, brush_name,
};

use crate::brush_dynamics::{curves, heading};
use crate::brush_settings::settings;
use crate::brush_strip::Strip;
use crate::components::{
    Placement, answer_row, choices, dialog, heading_button, hint, text_button, text_choice_wide,
    text_field,
};
use crate::design::{INK, SPACE, TOUCH_TARGET};

/// The width of the Set row's word "Set", left of its choices, in points.
const SET_LABEL: f32 = 6.0 * SPACE;

/// Renaming the brush: its name as typed so far.
#[derive(Debug)]
struct Rename {
    text: String,
    /// The field takes the focus in the next pass.
    focus: bool,
    from: Rect,
}

/// The editor's own state while it shows.
#[derive(Debug, Default)]
pub(crate) struct BrushEditor {
    pub(crate) strip: Strip,
    /// The dynamics page shown instead of the settings.
    dynamics: Option<Dynamic>,
    rename: Option<Rename>,
    /// Asking before the copy is deleted: where Delete was tapped.
    deleting: Option<Rect>,
    /// The set the Set row moved the brush to this pass, for the list to show (S47).
    pub(crate) moved: Option<BrushSet>,
}

impl BrushEditor {
    /// The page inside the panel; its changes pushed to `taps`. Returns whether Back left the
    /// editor for the list.
    pub(crate) fn page(
        &mut self,
        ui: &mut egui::Ui,
        brush: &BrushProjection,
        taps: &mut Vec<Intent>,
    ) -> bool {
        let edited = &brush.edited;
        let (named, back) = ui
            .horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                // The name's words line up with every panel's heading, two spaces in; its
                // press wash takes the space before them.
                let named = match self.dynamics {
                    Some(dynamic) => {
                        ui.add_space(2.0 * SPACE);
                        ui.heading(heading(dynamic));
                        None
                    }
                    None => Some(heading_button(ui, &edited.name)),
                };
                let back = ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(SPACE);
                    text_button(ui, "Back", true).clicked()
                });
                (named, back.inner)
            })
            .inner;
        if let Some(named) = named.filter(egui::Response::clicked) {
            self.rename = Some(Rename {
                text: edited.name.clone(),
                focus: true,
                from: named.rect,
            });
        }
        if self.dynamics.is_none()
            && let Some(set) = set_row(ui, brush.set)
        {
            push(taps, BrushIntent::MoveToSet(set));
            self.moved = Some(set);
        }
        self.strip.show(ui);
        ui.add_space(SPACE);
        let page = self.dynamics;
        let opened = column(ui, brush, page, taps);
        if page.is_none() {
            self.footer(ui, brush, taps);
        }
        if let Some(dynamic) = opened {
            self.dynamics = Some(dynamic);
        }
        match (back, page) {
            (true, Some(_)) => {
                self.dynamics = None;
                false
            }
            (back, _) => back,
        }
    }

    /// Duplicate, then Reset for a built-in or Delete for a copy.
    fn footer(&mut self, ui: &mut egui::Ui, brush: &BrushProjection, taps: &mut Vec<Intent>) {
        let edited = &brush.edited;
        ui.separator();
        ui.horizontal(|ui| {
            ui.set_height(TOUCH_TARGET);
            ui.add_space(SPACE);
            if text_button(ui, "Duplicate", true).clicked() {
                push(taps, BrushIntent::Duplicate);
            }
            if edited.built_in {
                if text_button(ui, "Reset", edited.resettable).clicked() {
                    push(taps, BrushIntent::Reset);
                }
            } else {
                let delete = text_button(ui, "Delete", true);
                if delete.clicked() {
                    self.deleting = Some(delete.rect);
                }
            }
        });
    }

    /// The Rename dialog or the question before deleting, over the screen. Returns whether one
    /// shows (it then answers every contact) and whether the brush was deleted.
    pub(crate) fn dialogs(
        &mut self,
        ctx: &egui::Context,
        brush: &BrushProjection,
        taps: &mut Vec<Intent>,
    ) -> (bool, bool) {
        if let Some(from) = self.deleting {
            let name = &brush.edited.name;
            let ((cancel, delete), outside) =
                dialog(ctx, "delete brush", (Placement::Centre, from), |ui| {
                    ui.heading(format!("Delete {name}?"));
                    hint(ui, "Strokes drawn with it stay.");
                    answer_row(ui, "Cancel", "Delete", true)
                });
            if delete {
                push(taps, BrushIntent::Delete);
            }
            if delete || cancel || outside {
                self.deleting = None;
            }
            return (true, delete);
        }
        let Some(rename) = &mut self.rename else {
            return (false, false);
        };
        let ((cancel, done), outside) =
            dialog(ctx, "rename brush", (Placement::Top, rename.from), |ui| {
                ui.heading("Rename brush");
                let field = text_field(ui, "Name", &mut rename.text, MAX_BRUSH_NAME, rename.focus);
                if field.response.has_focus() {
                    rename.focus = false;
                }
                let valid = brush_name(&rename.text).is_some();
                let (cancel, done) = answer_row(ui, "Cancel", "Done", valid);
                (cancel, done || (field.submitted && valid))
            });
        if done {
            push(taps, BrushIntent::Rename(rename.text.clone()));
        }
        if done || cancel || outside {
            self.rename = None;
        }
        (true, false)
    }

    /// The back button: a dialog closes, else a dynamics page goes back to the settings, else
    /// the editor goes back to the list. Returns whether the editor stays.
    pub(crate) fn back(&mut self) -> bool {
        if self.rename.take().is_some() || self.deleting.take().is_some() {
            return true;
        }
        self.dynamics.take().is_some()
    }
}

/// The settings, or `page`'s curves, in one column a finger scrolls, down to two rows above the
/// screen's bottom. Returns the dynamics page a row opened.
fn column(
    ui: &mut egui::Ui,
    brush: &BrushProjection,
    page: Option<Dynamic>,
    taps: &mut Vec<Intent>,
) -> Option<Dynamic> {
    let room = (ui.ctx().content_rect().bottom() - ui.cursor().top() - 2.0 * TOUCH_TARGET)
        .max(2.0 * TOUCH_TARGET);
    // A room of its own: the panel's area lays out within the size it had last, so a short
    // list shown before (a set's, S47) would keep the column short.
    let size = vec2(ui.available_width(), room);
    ui.allocate_ui(size, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(page.map_or("brush settings", heading))
            .max_height(room)
            // A finger drags the column (egui asks for a touch screen otherwise).
            .scroll_source(egui::scroll_area::ScrollSource::ALL)
            .show(ui, |ui| match page {
                Some(dynamic) => {
                    curves(ui, dynamic, &brush.edited, taps);
                    None
                }
                None => settings(ui, brush, taps),
            })
            .inner
    })
    .inner
}

/// The brush's set (S47): "Set", then the four sets as words in two columns, `set` in the ink
/// pill. Returns another set tapped, which the brush moves to the end of.
pub(crate) fn set_row(ui: &mut egui::Ui, set: BrushSet) -> Option<BrushSet> {
    let half = ((ui.available_width() - 3.0 * SPACE - SET_LABEL) / 2.0).floor();
    let mut tapped = None;
    choices(ui, "brush set", |ui, group| {
        for (line, pair) in BrushSet::EDITABLE.chunks(2).enumerate() {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                let size = vec2(SET_LABEL, TOUCH_TARGET);
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                if line == 0 {
                    let font = TextStyle::Body.resolve(ui.style());
                    let at = rect.left_center();
                    ui.painter().text(at, Align2::LEFT_CENTER, "Set", font, INK);
                }
                for &word in pair {
                    let on = word == set;
                    let choice = text_choice_wide(ui, group, word.label(), (true, on), half);
                    if choice.clicked() && !on {
                        tapped = Some(word);
                    }
                }
            });
        }
    });
    tapped
}

/// egui may run a pass twice; one tap is sent once.
fn push(taps: &mut Vec<Intent>, intent: BrushIntent) {
    let intent = Intent::Brush(intent);
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}
