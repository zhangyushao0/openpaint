//! The gallery's dialogs, each grown out of its card: Rename (a text field, Cancel and
//! Done, under the islands above the soft keyboard), Delete (it cannot be undone, so it asks)
//! and a card's options (save as an OpenPaint file, kept versions).

use egui::{Align, Layout};
use openpaint_app::{
    ARTWORK_EXTENSION, GalleryIntent, Intent, MAX_TITLE_CHARS, ProjectSummary, Title,
};

use super::dates::kept_labels;
use super::{HomeScreen, shown_title};
use crate::components::{
    Placement, delete_dialog, dialog, hint, menu_row, naming_dialog, section, text_button,
};
use crate::design::SPACE;
use crate::text::{self as t, Words};

impl HomeScreen {
    /// The title in a text field with Cancel and Done; Done (or Enter) renames when the text
    /// is a valid title.
    pub(super) fn rename_dialog(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let Some((id, naming)) = &mut self.rename else {
            return;
        };
        let words = (t::RENAME, t::TITLE, MAX_TITLE_CHARS);
        let valid = |text: &str| Title::new(text).is_ok();
        let Some(done) = naming_dialog(ctx, "rename", words, naming, valid) else {
            return;
        };
        if done && let Ok(title) = Title::new(&naming.text) {
            let id = *id;
            taps.push(Intent::Gallery(GalleryIntent::Rename { id, title }));
        }
        self.rename = None;
    }

    pub(super) fn confirm_dialog(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let Some((id, title, card)) = &self.confirm else {
            return;
        };
        let language = t::language(ctx);
        let question = t::delete_artwork(language, shown_title(ctx, title).as_str());
        let note = |ui: &mut egui::Ui| {
            ui.label(t::CANNOT_BE_UNDONE.get(language));
        };
        let Some(delete) = delete_dialog(ctx, "delete", *card, &question, note) else {
            return;
        };
        if delete {
            taps.push(Intent::Gallery(GalleryIntent::Delete(*id)));
        }
        self.confirm = None;
    }
}

impl HomeScreen {
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
        let language = t::language(ctx);
        let shown = shown_title(ctx, &item.title);
        let ((chosen, close), outside) = dialog(ctx, "options", (Placement::Top, card), |ui| {
            ui.heading(shown.as_str());
            let mut chosen = None;
            let extension = format!(".{ARTWORK_EXTENSION}");
            if menu_row(ui, t::SAVE_AS_FILE, Words::made(&extension), true).clicked() {
                chosen = Some(GalleryIntent::SaveFile(id));
            }
            section(ui, t::VERSIONS);
            if item.versions.is_empty() {
                hint(ui, t::KEPT_WHEN_LEFT);
            } else {
                hint(ui, t::OPENS_AS_COPY);
            }
            let times: Vec<u64> = item
                .versions
                .iter()
                .map(|version| version.kept_at)
                .collect();
            let labels = kept_labels(&times, offset, language);
            for (version, (name, label)) in item.versions.iter().zip(labels) {
                if menu_row(ui, Words::named(&name, &label), Words::EMPTY, true).clicked() {
                    chosen = Some(GalleryIntent::Restore {
                        id,
                        version: version.id,
                        title: shown.noted(&label),
                    });
                }
            }
            ui.add_space(SPACE);
            let close = ui
                .with_layout(Layout::right_to_left(Align::Center), |ui| {
                    text_button(ui, t::CLOSE, true).clicked()
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
