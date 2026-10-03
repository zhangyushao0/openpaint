//! The gallery screen (Procreate's model): the top bar with the "+" that makes a new artwork of
//! a size chosen in a small popover, and below it every artwork as a card in a grid, the one
//! changed last first. Tap a picture to
//! open it, its title to rename it (a dialog under the top bar, above the soft keyboard), the
//! copy button to duplicate it and the bin to delete it, which asks first: it cannot be undone.

use std::collections::HashMap;

use egui::{Align, Layout, Vec2};
use openpaint_app::{
    CanvasPreset, GalleryIntent, GalleryNotice, GalleryProjection, Intent, MAX_TITLE_CHARS,
    ProjectId, ProjectSummary, Thumbnail, Title,
};

use crate::components::{
    ArtworkCard, Placement, ScreenArea, artwork_card, card_size, dialog, icon_button, icon_toggle,
    menu_row, notice, popover, text_button, text_field,
};
use crate::design::{CARD_GAP, Icon, SPACE, TOUCH_TARGET};

/// The sizes "+" offers, top to bottom.
const PRESETS: [(CanvasPreset, &str); 4] = [
    (CanvasPreset::Screen, "Screen size"),
    (CanvasPreset::Square, "Square"),
    (CanvasPreset::A4Portrait, "A4 portrait"),
    (CanvasPreset::A4Landscape, "A4 landscape"),
];

/// The new-canvas popover.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Sizes {
    #[default]
    Closed,
    /// Opened by this frame's tap, which must not close it again.
    Opened,
    Shown,
}

/// The gallery's UI-local state.
#[derive(Default)]
pub(crate) struct GalleryScreen {
    sizes: Sizes,
    /// One texture per listed thumbnail, replaced when the thumbnail changes.
    pictures: HashMap<ProjectId, (Thumbnail, egui::TextureHandle)>,
    rename: Option<Rename>,
    /// The artwork whose deletion waits for the user's answer.
    confirm: Option<(ProjectId, Title)>,
}

impl std::fmt::Debug for GalleryScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GalleryScreen")
            .field("sizes", &self.sizes)
            .field("pictures", &self.pictures.len())
            .field("rename", &self.rename)
            .field("confirm", &self.confirm)
            .finish()
    }
}

#[derive(Debug)]
struct Rename {
    id: ProjectId,
    text: String,
    /// True until the field has taken the focus.
    focus: bool,
}

impl GalleryScreen {
    /// The top bar's contents: the screen's name, and at the right end "+" with any notice.
    /// "+" opens the new-canvas popover (and closes it again).
    pub(crate) fn bar(
        &mut self,
        ui: &mut egui::Ui,
        projection: &GalleryProjection,
        taps: &mut Vec<Intent>,
    ) {
        ui.label("Gallery");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let enabled = projection.availability().is_available();
            let label = "New artwork";
            let open = self.sizes != Sizes::Closed;
            let tapped = if enabled {
                icon_toggle(ui, Icon::Add, label, open).clicked()
            } else {
                icon_button(ui, Icon::Add, label, false).clicked()
            };
            if tapped {
                self.sizes = if open { Sizes::Closed } else { Sizes::Opened };
            }
            if let Some(message) = message(projection.notice())
                && notice(ui, message, "OK", true).clicked()
            {
                taps.push(Intent::Gallery(GalleryIntent::Dismiss));
            }
        });
    }

    /// The grid of cards inside `ui` (the area under the top bar), and an open dialog.
    pub(crate) fn body(
        &mut self,
        ui: &mut egui::Ui,
        projection: &GalleryProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.load_pictures(ui.ctx(), projection.items());
        // While the sizes are open a tap elsewhere only closes them.
        let enabled = projection.availability().is_available() && self.sizes == Sizes::Closed;
        if projection.listed() && projection.items().is_empty() {
            ui.centered_and_justified(|ui| ui.label("No artworks yet. Tap + to start one."));
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.grid(ui, projection.items(), enabled, taps);
            });
        }
        self.rename_dialog(ui.ctx(), taps);
        self.confirm_dialog(ui.ctx(), taps);
        let screen = ScreenArea::from_points(ui.max_rect(), ui.ctx().pixels_per_point());
        self.sizes_popover(ui.ctx(), screen.size, taps);
    }

    /// "New canvas": one row per size, its pixels at the right end. A row makes the artwork;
    /// a tap outside closes the popover, and the tap that opened it does not.
    fn sizes_popover(&mut self, ctx: &egui::Context, area: [u32; 2], taps: &mut Vec<Intent>) {
        if self.sizes == Sizes::Closed {
            return;
        }
        let mut chosen = None;
        let rect = popover(ctx, "new canvas", |ui| {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading("New canvas");
            });
            for (preset, label) in PRESETS {
                let [width, height] = preset.size(area);
                let note = format!("{width} × {height}");
                if menu_row(ui, label, &note, true).clicked() {
                    chosen = Some(preset);
                }
            }
        });
        let released_outside = ctx.input(|input| {
            input.pointer.any_released()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|at| !rect.contains(at))
        });
        if let Some(preset) = chosen {
            taps.push(Intent::Gallery(GalleryIntent::Create(preset)));
            self.sizes = Sizes::Closed;
        } else if self.sizes == Sizes::Opened {
            self.sizes = Sizes::Shown;
        } else if released_outside {
            self.sizes = Sizes::Closed;
        }
    }

    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        items: &[ProjectSummary],
        enabled: bool,
        taps: &mut Vec<Intent>,
    ) {
        // Left-aligned rows, `CARD_GAP` around and between the cards.
        let pitch = card_size() + Vec2::splat(CARD_GAP);
        let width = ui.available_width();
        let columns = (((width - CARD_GAP) / pitch.x).floor() as usize).max(1);
        let rows = items.len().div_ceil(columns);
        let origin = ui.cursor().min + Vec2::splat(CARD_GAP);
        let height = rows as f32 * pitch.y + CARD_GAP;
        ui.allocate_space(Vec2::new(width, height));
        for (index, item) in items.iter().enumerate() {
            let cell = Vec2::new((index % columns) as f32, (index / columns) as f32);
            let rect = egui::Rect::from_min_size(origin + cell * pitch, card_size());
            let mut slot = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            self.card(&mut slot, item, enabled, taps);
        }
    }

    fn card(
        &mut self,
        ui: &mut egui::Ui,
        item: &ProjectSummary,
        enabled: bool,
        taps: &mut Vec<Intent>,
    ) {
        let picture = self.pictures.get(&item.id).map(|(_, texture)| texture);
        let tapped = artwork_card(
            ui,
            &ArtworkCard {
                id: egui::Id::new(("artwork", item.id.get())),
                title: item.title.as_str(),
                picture,
                enabled,
            },
        );
        if tapped.open {
            taps.push(Intent::Gallery(GalleryIntent::Open(item.id)));
        } else if tapped.rename {
            self.rename = Some(Rename {
                id: item.id,
                text: item.title.as_str().to_owned(),
                focus: true,
            });
        } else if tapped.duplicate {
            taps.push(Intent::Gallery(GalleryIntent::Duplicate(item.id)));
        } else if tapped.delete {
            self.confirm = Some((item.id, item.title.clone()));
        }
    }

    /// The title in a text field with Cancel and Done; Done (or Enter) renames when the text
    /// is a valid title.
    fn rename_dialog(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let Some(rename) = &mut self.rename else {
            return;
        };
        let ((cancel, done), outside) = dialog(ctx, "rename", Placement::Top, |ui| {
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

    fn confirm_dialog(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let Some((id, title)) = &self.confirm else {
            return;
        };
        let ((cancel, delete), outside) = dialog(ctx, "delete", Placement::Centre, |ui| {
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

    /// Closes an open dialog as Cancel would, or the new-canvas popover. Returns whether one
    /// was open.
    pub(crate) fn back(&mut self) -> bool {
        let open = self.rename.is_some() || self.confirm.is_some() || self.sizes != Sizes::Closed;
        self.rename = None;
        self.confirm = None;
        self.sizes = Sizes::Closed;
        open
    }

    /// Uploads thumbnails that are new or changed, and lets go of those no longer listed.
    fn load_pictures(&mut self, ctx: &egui::Context, items: &[ProjectSummary]) {
        self.pictures
            .retain(|id, _| items.iter().any(|item| item.id == *id));
        for item in items {
            let Some(thumbnail) = &item.thumbnail else {
                self.pictures.remove(&item.id);
                continue;
            };
            if self
                .pictures
                .get(&item.id)
                .is_some_and(|(shown, _)| shown == thumbnail)
            {
                continue;
            }
            let [width, height] = thumbnail.size().map(|side| side as usize);
            let image =
                egui::ColorImage::from_rgba_unmultiplied([width, height], thumbnail.pixels());
            let name = format!("thumbnail-{}", item.id.get());
            let texture = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
            self.pictures.insert(item.id, (thumbnail.clone(), texture));
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

fn message(notice: GalleryNotice) -> Option<&'static str> {
    match notice {
        GalleryNotice::None => None,
        GalleryNotice::ListFailed => Some("Couldn't read the gallery."),
        GalleryNotice::OpenFailed => Some("Couldn't open this artwork. It is kept as it was."),
        GalleryNotice::CreateFailed => Some("Couldn't make a new artwork."),
        GalleryNotice::ChangeFailed => Some("Couldn't change the gallery."),
    }
}
