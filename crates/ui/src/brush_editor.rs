//! The brush editor (Procreate's Brush Studio reduced to the engine's settings): tapping the
//! chosen brush's row again turns the brush panel into it in place, as Actions turns into the
//! Pen buttons page. At the top the brush's name (tapped: the Rename dialog) and Back; under it
//! the brush's set (Sketching, Inking, Painting, Texture as words, its own in the ink pill;
//! another tapped moves it to that set's end); then the preview strip (`brush_strip.rs`); then
//! the settings in one scrolling column (`brush_settings.rs`), or a dynamics page's curves
//! (`brush_dynamics.rs`, Back to the settings); at the bottom Duplicate, and Reset for a
//! built-in (grey while it is as its file has it) or Delete for a copy (asked first: it cannot
//! be undone). Every change applies to the next stroke of every tool that draws with the brush;
//! strokes drawn keep their look; none is an undo step, as colours are not.

use egui::{Align, Layout, Rect, vec2};
use openpaint_app::{
    BrushIntent, BrushProjection, BrushSet, Dynamic, Intent, MAX_BRUSH_NAME, brush_name,
};

use crate::brush_dynamics::curves;
use crate::brush_settings::settings;
use crate::brush_strip::Strip;
use crate::components::{
    Lead, Naming, delete_dialog, heading_button, hint, naming_dialog, text_button, word_columns,
};
use crate::design::{SPACE, TOUCH_TARGET};
use crate::taps::push_brush;
use crate::text::{self as t, Words, dynamic as dynamic_text};

/// The width of the Set row's word "Set", left of its choices, in points.
const SET_LABEL: f32 = 6.0 * SPACE;

/// The editor's own state while it shows.
#[derive(Debug, Default)]
pub(crate) struct BrushEditor {
    pub(crate) strip: Strip,
    /// The dynamics page shown instead of the settings.
    dynamics: Option<Dynamic>,
    /// Renaming the brush: its name as typed so far, and as it showed when the dialog opened
    /// (a built-in's name shows in the language: kept unless it is changed).
    rename: Option<(Naming, String)>,
    /// Asking before the copy is deleted: where Delete was tapped.
    deleting: Option<Rect>,
    /// The set the Set row moved the brush to this pass, for the list to show.
    pub(crate) moved: Option<BrushSet>,
}

impl BrushEditor {
    /// The page inside the panel; its changes pushed to `taps`. Returns whether Back left the
    /// editor for the list.
    pub(crate) fn page(
        &mut self,
        ui: &mut egui::Ui,
        (brush, streamline): (&BrushProjection, bool),
        taps: &mut Vec<Intent>,
    ) -> bool {
        let edited = &brush.edited;
        let name = t::shown_name(ui.ctx(), &edited.name);
        let (named, back) = ui
            .horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                // The name's words line up with every panel's heading, two spaces in; its
                // press wash takes the space before them.
                let named = match self.dynamics {
                    Some(dynamic) => {
                        ui.add_space(2.0 * SPACE);
                        ui.heading(Words::from(dynamic_text(dynamic)).shown(ui.ctx()));
                        None
                    }
                    None => Some(heading_button(ui, Words::named(&edited.name, &name))),
                };
                let back = ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(SPACE);
                    text_button(ui, t::BACK, true).clicked()
                });
                (named, back.inner)
            })
            .inner;
        if let Some(named) = named.filter(egui::Response::clicked) {
            self.rename = Some((Naming::new(&name, named.rect), name.into_owned()));
        }
        if self.dynamics.is_none()
            && let Some(set) = set_row(ui, brush.set)
        {
            push_brush(taps, BrushIntent::MoveToSet(set));
            self.moved = Some(set);
        }
        self.strip.show(ui);
        ui.add_space(SPACE);
        let page = self.dynamics;
        let opened = column(ui, (brush, streamline), page, taps);
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
            if text_button(ui, t::DUPLICATE, true).clicked() {
                push_brush(taps, BrushIntent::Duplicate);
            }
            if edited.built_in {
                if text_button(ui, t::RESET, edited.resettable).clicked() {
                    push_brush(taps, BrushIntent::Reset);
                }
            } else {
                let delete = text_button(ui, t::DELETE, true);
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
            let language = t::language(ctx);
            let name = t::shown_name(ctx, &brush.edited.name);
            let question = t::delete_brush(language, &name);
            let note = |ui: &mut egui::Ui| hint(ui, t::STROKES_STAY);
            let answer = delete_dialog(ctx, "delete brush", from, &question, note);
            let delete = answer == Some(true);
            if delete {
                push_brush(taps, BrushIntent::Delete);
            }
            if answer.is_some() {
                self.deleting = None;
            }
            return (true, delete);
        }
        let Some((naming, shown)) = &mut self.rename else {
            return (false, false);
        };
        let words = (t::RENAME_BRUSH, t::NAME, MAX_BRUSH_NAME);
        let valid = |text: &str| brush_name(text).is_some();
        if let Some(done) = naming_dialog(ctx, "rename brush", words, naming, valid) {
            if done && naming.text != *shown {
                push_brush(taps, BrushIntent::Rename(naming.text.clone()));
            }
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

/// The settings (StreamLine among them while it applies), or `page`'s curves, in one column a
/// finger scrolls, down to two rows above the screen's bottom. Returns the dynamics page a row
/// opened.
fn column(
    ui: &mut egui::Ui,
    (brush, streamline): (&BrushProjection, bool),
    page: Option<Dynamic>,
    taps: &mut Vec<Intent>,
) -> Option<Dynamic> {
    let room = (ui.ctx().content_rect().bottom() - ui.cursor().top() - 2.0 * TOUCH_TARGET)
        .max(2.0 * TOUCH_TARGET);
    // A room of its own: the panel's area lays out within the size it had last, so a short
    // list shown before (a set's) would keep the column short.
    let size = vec2(ui.available_width(), room);
    ui.allocate_ui(size, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(page.map_or("brush settings", |page| dynamic_text(page).id()))
            .max_height(room)
            // A finger drags the column (egui asks for a touch screen otherwise).
            .scroll_source(egui::scroll_area::ScrollSource::ALL)
            .show(ui, |ui| match page {
                Some(dynamic) => {
                    curves(ui, dynamic, &brush.edited, taps);
                    None
                }
                None => settings(ui, (brush, streamline), taps),
            })
            .inner
    })
    .inner
}

/// The brush's set: "Set", then the four sets as words in two columns, `set` in the ink
/// pill. Returns another set tapped, which the brush moves to the end of.
pub(crate) fn set_row(ui: &mut egui::Ui, set: BrushSet) -> Option<BrushSet> {
    let words = BrushSet::EDITABLE.map(|word| (t::brush_set(word).into(), word == set, true));
    let lead = Lead::Label(t::SET.into(), SET_LABEL);
    word_columns(ui, "brush set", &words, lead).map(|at| BrushSet::EDITABLE[at])
}
