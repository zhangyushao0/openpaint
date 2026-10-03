//! The gallery's two dialogs, each grown out of its card: Rename (a text field, Cancel and
//! Done, under the islands above the soft keyboard) and Delete (it cannot be undone, so it
//! asks).

use egui::{Align, Layout};
use openpaint_app::{GalleryIntent, Intent, MAX_TITLE_CHARS, Title};

use super::GalleryScreen;
use crate::components::{Placement, dialog, text_button, text_field};
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
            let field = text_field(
                ui,
                "rename-field",
                &mut rename.text,
                MAX_TITLE_CHARS,
                rename.focus,
            );
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
            ui.label("This can't be undone.");
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
