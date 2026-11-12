//! The studio's entrance on the home (S122): "Studio" over the cards the search finds, in a grid,
//! the one changed last first, or why there are none; a card's taps open, rename, show the
//! options of, duplicate or delete its artwork.

use egui::{Align2, Rect, TextStyle, Vec2};
use openpaint_app::{GalleryIntent, GalleryProjection, Intent, ProjectSummary};

use super::{HomeScreen, shown_title};
use crate::components::carry::{Opening, land};
use crate::components::{ArtworkCard, Naming, artwork_card, card_size};
use crate::design::{CARD_GAP, INK, SPACE, TOUCH_TARGET};
use crate::text::{self as t, Words};

impl HomeScreen {
    /// The studio in `rect`: its name, then the cards the search finds (or why there are none).
    pub(super) fn studio(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        (projection, enabled): (&GalleryProjection, bool),
        taps: &mut Vec<Intent>,
    ) {
        let ctx = ui.ctx().clone();
        let head = Rect::from_min_size(
            rect.min + Vec2::new(CARD_GAP, SPACE),
            Vec2::new(rect.width() - CARD_GAP, TOUCH_TARGET),
        );
        let font = TextStyle::Heading.resolve(ui.style());
        let name = Words::from(t::STUDIO).shown(&ctx);
        ui.painter()
            .text(head.left_center(), Align2::LEFT_CENTER, name, font, INK);
        let below = Rect::from_min_max(egui::pos2(rect.left(), head.bottom()), rect.max);
        let mut list = ui.new_child(egui::UiBuilder::new().max_rect(below));
        let found: Vec<&ProjectSummary> = (projection.items().iter())
            .filter(|item| shown_title(&ctx, &item.title).matches(&self.search))
            .collect();
        if projection.listed() && projection.items().is_empty() {
            self.arriving = self.arriving.filter(|arrival| arrival.card);
            let empty = Words::from(t::NO_ARTWORKS).shown(&ctx);
            list.centered_and_justified(|ui| ui.label(empty));
        } else if found.is_empty() && !projection.items().is_empty() {
            let text = t::nothing_found(t::language(&ctx), self.search.trim());
            list.centered_and_justified(|ui| ui.label(text));
        } else {
            egui::ScrollArea::vertical().show(&mut list, |ui| {
                self.grid(ui, &found, enabled, taps);
            });
        }
    }

    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        items: &[&ProjectSummary],
        enabled: bool,
        taps: &mut Vec<Intent>,
    ) {
        // Left-aligned rows, `CARD_GAP` around and between the cards.
        let pitch = card_size() + Vec2::splat(CARD_GAP);
        let width = ui.available_width();
        let columns = (((width - CARD_GAP) / pitch.x).floor() as usize).max(1);
        let rows = items.len().div_ceil(columns);
        let origin = ui.cursor().min + Vec2::new(CARD_GAP, SPACE);
        let height = rows as f32 * pitch.y + CARD_GAP;
        ui.allocate_space(Vec2::new(width, height));
        let place = |slot: usize| {
            let cell = Vec2::new((slot % columns) as f32, (slot / columns) as f32);
            egui::Rect::from_min_size(origin + cell * pitch, card_size())
        };
        // The card the canvas comes back into: its own, else the first (changed last).
        let arriving = self
            .arriving
            .filter(|arrival| !arrival.card)
            .map(|arrival| {
                let listed = items.iter().any(|item| Some(item.id) == arrival.id);
                let id = arrival
                    .id
                    .filter(|_| listed)
                    .or(items.first().map(|item| item.id));
                (arrival, id)
            });
        for (index, item) in items.iter().enumerate() {
            let rect = place(index);
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
        let title = t::shown_name(ui.ctx(), item.title.as_str());
        let tapped = artwork_card(
            ui,
            &ArtworkCard {
                id: egui::Id::new(("artwork", item.id.get())),
                title: &title,
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
            self.rename = Some((item.id, Naming::new(&title, card)));
        } else if tapped.options {
            self.options = Some((item.id, card));
        } else if tapped.duplicate {
            taps.push(Intent::Gallery(GalleryIntent::Duplicate(item.id)));
        } else if tapped.delete {
            self.confirm = Some((item.id, item.title.clone(), card));
        }
        tapped.picture
    }
}
