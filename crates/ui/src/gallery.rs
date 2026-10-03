//! The gallery screen (Procreate's model): its name on the desk at the top left, an island at
//! the top right with the "+" that makes a new artwork of a size chosen in a small popover, and
//! below them every artwork as a card in a grid, the one changed last first. Tap a picture to
//! open it, its title to rename it (a dialog under the islands, above the soft keyboard), the
//! copy button to duplicate it and the bin to delete it, which asks first: it cannot be undone.

#[path = "gallery_dialogs.rs"]
mod dialogs;

use std::collections::HashMap;

use egui::{Align, Layout, Rect, Vec2};
use openpaint_app::{
    CanvasPreset, GalleryIntent, GalleryNotice, GalleryProjection, Intent, ProjectId,
    ProjectSummary, Thumbnail, Title,
};

use crate::components::carry::{Arrival, Opening, land};
use crate::components::{
    ArtworkCard, ISLAND_TOP, Opener, Place, ScreenArea, Source, TOP_BAND, artwork_card, card_size,
    icon_button, icon_toggle, island, menu_row, notice, pill, popover,
};
use crate::design::{CARD_GAP, Icon, SPACE, TOUCH_TARGET};

/// The island "+" sits on; the new-canvas popover grows out of it.
const ISLAND: &str = "gallery island";

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
    /// The artwork whose deletion waits for the user's answer, and its card (points).
    confirm: Option<(ProjectId, Title, Rect)>,
    /// Where "+" lies, in points, from this pass.
    plus: Option<Rect>,
    /// The artwork just opened or made, until the canvas takes it.
    opening: Option<Opening>,
    /// The artwork coming back from the canvas into its card.
    arriving: Option<Arrival>,
    arrivals: u64,
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
    /// The card it came from, in points.
    card: Rect,
}

impl GalleryScreen {
    /// The artwork just opened or made, once: the canvas grows out of where it lay.
    pub(crate) fn take_opening(&mut self) -> Option<Opening> {
        self.opening.take()
    }

    /// The canvas, last shown at `from` (points), comes back into the card of `id` (or the
    /// first card).
    pub(crate) fn arrive(&mut self, from: Rect, id: Option<ProjectId>) {
        self.arrivals += 1;
        self.arriving = Some(Arrival {
            from,
            id,
            run: self.arrivals,
        });
    }

    /// The top band: the screen's name on the desk at the left, and at the right an island with
    /// "+" and, left of it, a pill with any notice. "+" opens the new-canvas popover (and closes
    /// it again).
    pub(crate) fn islands(
        &mut self,
        ctx: &egui::Context,
        projection: &GalleryProjection,
        taps: &mut Vec<Intent>,
    ) {
        egui::Area::new(egui::Id::new("gallery title"))
            .order(egui::Order::Background)
            .interactable(false)
            .fade_in(false)
            .anchor(egui::Align2::LEFT_TOP, Vec2::new(2.0 * SPACE, ISLAND_TOP))
            .show(ctx, |ui| {
                ui.set_height(TOUCH_TARGET);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.heading("Gallery");
                });
            });
        let enabled = projection.availability().is_available();
        let open = self.sizes != Sizes::Closed;
        let (button, plus) = island(ctx, ISLAND, Place::top_right(0.0), |ui| {
            let label = "New artwork";
            if enabled {
                icon_toggle(ui, Icon::Add, label, open)
            } else {
                icon_button(ui, Icon::Add, label, false)
            }
        });
        self.plus = Some(button.rect);
        if button.clicked() {
            self.sizes = if open { Sizes::Closed } else { Sizes::Opened };
        }
        if let Some(message) = message(projection.notice()) {
            let place = Place::top_right(plus.width() + SPACE);
            let source = Source::Within(Align::Max);
            let (dismissed, _) = pill(ctx, "gallery notice", place, source, |ui| {
                notice(ui, message, "OK", true).clicked()
            });
            if dismissed {
                taps.push(Intent::Gallery(GalleryIntent::Dismiss));
            }
        }
    }

    /// The grid of cards inside `ui` (the whole screen, the cards under the top band), and an
    /// open dialog.
    pub(crate) fn body(
        &mut self,
        ui: &mut egui::Ui,
        projection: &GalleryProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.load_pictures(ui.ctx(), projection.items());
        // While the sizes are open a tap elsewhere only closes them.
        let enabled = projection.availability().is_available() && self.sizes == Sizes::Closed;
        let mut under = ui.max_rect();
        under.min.y += TOP_BAND;
        let mut list = ui.new_child(egui::UiBuilder::new().max_rect(under));
        if projection.listed() && projection.items().is_empty() {
            self.arriving = None;
            list.centered_and_justified(|ui| ui.label("No artworks yet. Tap + to start one."));
        } else {
            egui::ScrollArea::vertical().show(&mut list, |ui| {
                self.grid(ui, projection.items(), enabled, taps);
            });
        }
        self.rename_dialog(ui.ctx(), taps);
        self.confirm_dialog(ui.ctx(), taps);
        // "Screen size" is the area a canvas fits in.
        let screen = ScreenArea::from_points(under, ui.ctx().pixels_per_point());
        self.sizes_popover(ui.ctx(), screen.size, taps);
    }

    /// "New canvas": one row per size, its pixels at the right end. A row makes the artwork;
    /// a tap outside closes the popover, and the tap that opened it does not.
    fn sizes_popover(&mut self, ctx: &egui::Context, area: [u32; 2], taps: &mut Vec<Intent>) {
        let mut chosen = None;
        let opener = Opener {
            island: ISLAND,
            button: self.plus.unwrap_or(Rect::ZERO),
        };
        let open = self.sizes != Sizes::Closed;
        let rect = popover(ctx, "new canvas", (opener, open), |ui| {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading("New canvas");
            });
            for (preset, label) in PRESETS {
                let [width, height] = preset.size(area);
                let note = format!("{width} × {height}");
                let row = menu_row(ui, label, &note, true);
                if row.clicked() {
                    chosen = Some((preset, row.rect));
                }
            }
        });
        let Some(rect) = rect else {
            return;
        };
        let released_outside = ctx.input(|input| {
            input.pointer.any_released()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|at| !rect.contains(at))
        });
        if let Some((preset, row)) = chosen {
            taps.push(Intent::Gallery(GalleryIntent::Create(preset)));
            self.opening = Some(Opening {
                from: row,
                id: None,
                picture: None,
            });
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
        // The card the canvas comes back into: its own, else the first (changed last).
        let arriving = self.arriving.map(|arrival| {
            let listed = items.iter().any(|item| Some(item.id) == arrival.id);
            let id = arrival
                .id
                .filter(|_| listed)
                .or(items.first().map(|item| item.id));
            (arrival, id)
        });
        for (index, item) in items.iter().enumerate() {
            let cell = Vec2::new((index % columns) as f32, (index / columns) as f32);
            let rect = egui::Rect::from_min_size(origin + cell * pitch, card_size());
            let mut slot = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            let landing = arriving.filter(|(_, id)| *id == Some(item.id));
            let card = self.card(&mut slot, item, (enabled, landing.is_some()), taps);
            if let Some((arrival, _)) = landing {
                let picture = self.pictures.get(&item.id).map(|(_, texture)| texture);
                if land(ui.ctx(), &arrival, picture, card) {
                    self.arriving = None;
                }
            }
        }
    }

    /// One card; returns where its picture lies.
    fn card(
        &mut self,
        ui: &mut egui::Ui,
        item: &ProjectSummary,
        (enabled, away): (bool, bool),
        taps: &mut Vec<Intent>,
    ) -> Rect {
        let picture = self.pictures.get(&item.id).map(|(_, texture)| texture);
        let tapped = artwork_card(
            ui,
            &ArtworkCard {
                id: egui::Id::new(("artwork", item.id.get())),
                title: item.title.as_str(),
                picture,
                enabled,
                away,
            },
        );
        let card = ui.max_rect();
        if tapped.open {
            taps.push(Intent::Gallery(GalleryIntent::Open(item.id)));
            self.opening = Some(Opening {
                from: tapped.picture,
                id: Some(item.id),
                picture: picture.cloned(),
            });
        } else if tapped.rename {
            self.rename = Some(Rename {
                id: item.id,
                text: item.title.as_str().to_owned(),
                focus: true,
                card,
            });
        } else if tapped.duplicate {
            taps.push(Intent::Gallery(GalleryIntent::Duplicate(item.id)));
        } else if tapped.delete {
            self.confirm = Some((item.id, item.title.clone(), card));
        }
        tapped.picture
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

fn message(notice: GalleryNotice) -> Option<&'static str> {
    match notice {
        GalleryNotice::None => None,
        GalleryNotice::ListFailed => Some("Couldn't read the gallery."),
        GalleryNotice::OpenFailed => Some("Couldn't open this artwork. It is kept as it was."),
        GalleryNotice::CreateFailed => Some("Couldn't make a new artwork."),
        GalleryNotice::ChangeFailed => Some("Couldn't change the gallery."),
    }
}
