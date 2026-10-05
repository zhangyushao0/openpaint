//! The gallery's dialogs, each grown out of its card: Rename (a text field, Cancel and
//! Done, under the islands above the soft keyboard), Delete (it cannot be undone, so it asks)
//! and a card's options (save as an OpenPaint file, kept versions).

use egui::{Align, Layout};
use openpaint_app::{GalleryIntent, Intent, MAX_TITLE_CHARS, ProjectSummary, Title};

use super::GalleryScreen;
use super::dates::kept_labels;
use crate::components::{Placement, dialog, hint, menu_row, section, text_button, text_field};
use crate::design::SPACE;

impl GalleryScreen {
    /// The title in a text field with Cancel and Done; Done (or Enter) renames when the text
    /// is a valid title.
    pub(super) fn rename_dialog(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let Some(rename) = &mut self.rename else {
            return;
        };
        let from = (Placement::Top, rename.card);
        let ((cancel, done), outside) = dialog(ctx, "rename", from, |ui| {
            ui.heading("Rename");
            let field = text_field(ui, "Title", &mut rename.text, MAX_TITLE_CHARS, rename.focus);
            if field.response.has_focus() {
                rename.focus = false;
            }
            let valid = Title::new(&rename.text).is_ok();
            let (cancel, done) = answer_row(ui, "Cancel", "Done", valid);
            (cancel, done || (field.submitted && valid))
        });
        if done && let Ok(title) = Title::new(&rename.text) {
            taps.push(Intent::Gallery(GalleryIntent::Rename {
                id: rename.id,
                title,
            }));
            self.rename = None;
        } else if cancel || outside {
            self.rename = None;
        }
    }

    pub(super) fn confirm_dialog(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let Some((id, title, card)) = &self.confirm else {
            return;
        };
        let from = (Placement::Centre, *card);
        let ((cancel, delete), outside) = dialog(ctx, "delete", from, |ui| {
            ui.heading(format!("Delete “{title}”?"));
            ui.label("This can't be undone. Its kept versions go with it.");
            answer_row(ui, "Cancel", "Delete", true)
        });
        if delete {
            taps.push(Intent::Gallery(GalleryIntent::Delete(*id)));
            self.confirm = None;
        } else if cancel || outside {
            self.confirm = None;
        }
    }
}

impl GalleryScreen {
    /// A card's options under the islands, grown from its card: "Save as OpenPaint file…",
    /// then its kept versions, the newest first, each dated; a version opens as a new copy
    /// titled with its date. Close or a tap outside closes it.
    pub(super) fn options_dialog(
        &mut self,
        ctx: &egui::Context,
        items: &[ProjectSummary],
        taps: &mut Vec<Intent>,
    ) {
        let Some((id, card)) = self.options else {
            return;
        };
        let Some(item) = items.iter().find(|item| item.id == id) else {
            self.options = None;
            return;
        };
        let offset = self.utc_offset;
        let ((chosen, close), outside) = dialog(ctx, "options", (Placement::Top, card), |ui| {
            ui.heading(item.title.as_str());
            let mut chosen = None;
            if menu_row(ui, "Save as OpenPaint file…", ".openpaint", true).clicked() {
                chosen = Some(GalleryIntent::SaveFile(id));
            }
            section(ui, "Versions");
            if item.versions.is_empty() {
                hint(ui, "Kept when you leave it after a change.");
            } else {
                hint(ui, "Each opens as a new copy.");
            }
            let times: Vec<u64> = item
                .versions
                .iter()
                .map(|version| version.kept_at)
                .collect();
            for (version, label) in item.versions.iter().zip(kept_labels(&times, offset)) {
                if menu_row(ui, &label, "", true).clicked() {
                    chosen = Some(GalleryIntent::Restore {
                        id,
                        version: version.id,
                        title: item.title.noted(&label),
                    });
                }
            }
            ui.add_space(SPACE);
            let close = ui
                .with_layout(Layout::right_to_left(Align::Center), |ui| {
                    text_button(ui, "Close", true).clicked()
                })
                .inner;
            (chosen, close)
        });
        if let Some(intent) = chosen {
            taps.push(Intent::Gallery(intent));
            self.options = None;
        } else if close || outside {
            self.options = None;
        }
    }
}

/// Two buttons at the right of a dialog's last row: the cancelling one, then the answering one.
fn answer_row(ui: &mut egui::Ui, cancel: &str, answer: &str, enabled: bool) -> (bool, bool) {
    ui.add_space(SPACE);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let answered = text_button(ui, answer, enabled).clicked();
        let cancelled = text_button(ui, cancel, true).clicked();
        (cancelled, answered)
    })
    .inner
}
