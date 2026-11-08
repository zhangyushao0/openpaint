//! The gallery screen (Procreate's model): its name on the desk at the top left, a search field
//! in the top band's middle, an island at the top right with the "+" that makes a new artwork
//! of a size chosen in a small popover or opens an OpenPaint file, and below them a mode's card
//! (`GalleryHook::card`; not while searching), then every artwork whose title holds the search's
//! text as a card in a grid, the one changed last first. Tap a
//! picture to open it, its title to rename it (a dialog under the islands, above the soft
//! keyboard), the options to save it as an OpenPaint file or restore a kept version, the copy
//! button to duplicate it and the bin to delete it, which asks first: it cannot be undone. A
//! mode's page lies over the gallery with an island of its own, its dialogs after the gallery's.

#[path = "gallery_dates.rs"]
mod dates;
#[path = "gallery_dialogs.rs"]
mod dialogs;
#[path = "gallery_new.rs"]
mod new;

use std::collections::HashMap;

use egui::{Align, Layout, Rect, Vec2};
use openpaint_app::{
    GalleryIntent, GalleryProjection, Intent, MAX_TITLE_CHARS, ProjectId, ProjectSummary,
    Thumbnail, Title,
};

use crate::components::carry::{Arrival, Opening, land};
use crate::components::{
    Alpha, ArtworkCard, ISLAND_TOP, Naming, Place, Source, TOP_BAND, artwork_card, card_size,
    icon_button, icon_toggle, island, notice, picture_texture, pill, text_field,
};
use crate::design::{CARD_GAP, Icon, SPACE, TOUCH_TARGET};
use crate::hooks::{GalleryHook, Page};
use crate::text::{self as t, Words};

/// A mode's part of the gallery this pass, and the gallery's taps.
type Taps<'a, 'h> = (Option<&'a mut (dyn GalleryHook + 'h)>, &'a mut Vec<Intent>);

/// The island "+" sits on; the new-canvas popover grows out of it.
const ISLAND: &str = "gallery island";
/// The search field's width in points.
const SEARCH_WIDTH: f32 = 360.0;

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
    /// The artwork being renamed and its title as typed so far, the dialog grown from its card.
    rename: Option<(ProjectId, Naming)>,
    /// The artwork whose deletion waits for the user's answer, and its card (points).
    confirm: Option<(ProjectId, openpaint_app::Title, Rect)>,
    /// The artwork whose options show, and its card (points).
    options: Option<(ProjectId, Rect)>,
    /// What the search field holds; not stored.
    search: String,
    /// The device's offset from UTC in minutes (`Shell::set_utc_offset`).
    pub(crate) utc_offset: i32,
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
            .field("options", &self.options)
            .field("search", &self.search)
            .finish()
    }
}

impl GalleryScreen {
    /// The artwork just opened or made, once: the canvas grows out of where it lay.
    pub(crate) fn take_opening(&mut self) -> Option<Opening> {
        self.opening.take()
    }

    /// The canvas, last shown at `from` (points), comes back into the card of `id` (or the
    /// first card), a mode's sheet into the mode's `card`.
    pub(crate) fn arrive(&mut self, from: Rect, (id, card): (Option<ProjectId>, bool)) {
        self.arrivals += 1;
        self.arriving = Some(Arrival {
            from,
            id,
            card,
            run: self.arrivals,
        });
    }

    /// What the mode opened (if anything) since it was last asked: the canvas grows out of it.
    fn opened_by(&mut self, hook: &mut (dyn GalleryHook + '_)) {
        if let Some(opening) = hook.opening() {
            self.opening = Some(opening);
        }
    }

    /// The top band: the screen's name on the desk at the left, the search field in the
    /// middle, and at the right an island with "+" and, left of it, a pill with any notice.
    /// "+" opens the new-canvas popover (and closes it again).
    pub(crate) fn islands(
        &mut self,
        ctx: &egui::Context,
        projection: &GalleryProjection,
        (mut hook, taps): Taps<'_, '_>,
    ) {
        // A mode's page takes the whole screen: its own island only.
        if hook.as_deref_mut().is_some_and(|hook| hook.island(ctx)) {
            return;
        }
        egui::Area::new(egui::Id::new("gallery title"))
            .order(egui::Order::Background)
            .interactable(false)
            .fade_in(false)
            .anchor(egui::Align2::LEFT_TOP, Vec2::new(2.0 * SPACE, ISLAND_TOP))
            .show(ctx, |ui| {
                ui.set_height(TOUCH_TARGET);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.heading(Words::from(t::GALLERY).shown(ui.ctx()));
                });
            });
        egui::Area::new(egui::Id::new("gallery search"))
            .fade_in(false)
            .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, ISLAND_TOP))
            .show(ctx, |ui| {
                ui.set_width(SEARCH_WIDTH);
                text_field(ui, t::SEARCH, &mut self.search, MAX_TITLE_CHARS, false);
            });
        let enabled = projection.availability().is_available();
        let open = self.sizes != Sizes::Closed;
        let (button, plus) = island(ctx, ISLAND, Place::top_right(0.0), |ui| {
            let label = t::NEW_ARTWORK;
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
        // The gallery's own notice first, then the mode's.
        let told = (t::gallery_notice(projection.notice()).map(|m| (m, true)))
            .or_else(|| Some((hook.as_deref()?.notice()?, false)));
        if let Some((message, own)) = told {
            let place = Place::top_right(plus.width() + SPACE);
            let source = Source::Within(Align::Max);
            let (dismissed, _) = pill(ctx, "gallery notice", place, source, |ui| {
                notice(ui, message, t::OK, true).clicked()
            });
            if dismissed && own {
                taps.push(Intent::Gallery(GalleryIntent::Dismiss));
            } else if let (true, Some(hook)) = (dismissed, hook) {
                hook.dismiss();
            }
        }
    }

    /// The grid of the cards the search finds inside `ui` (the whole screen, the cards under
    /// the top band), and an open dialog; a mode's page over them while it shows.
    pub(crate) fn body(
        &mut self,
        ui: &mut egui::Ui,
        projection: &GalleryProjection,
        (mut hook, taps): Taps<'_, '_>,
    ) {
        let ctx = ui.ctx().clone();
        if let Some(hook) = hook.as_deref_mut() {
            let page = hook.page(&ctx, taps);
            self.opened_by(hook);
            if page == Page::Covers {
                return;
            }
        }
        let language = t::language(&ctx);
        self.load_pictures(&ctx, projection.items());
        // While the sizes are open a tap elsewhere only closes them.
        let enabled = projection.availability().is_available() && self.sizes == Sizes::Closed;
        let mut under = ui.max_rect();
        under.min.y += TOP_BAND;
        let mut list = ui.new_child(egui::UiBuilder::new().max_rect(under));
        let found: Vec<&ProjectSummary> = projection
            .items()
            .iter()
            .filter(|item| shown_title(&ctx, &item.title).matches(&self.search))
            .collect();
        // The mode's card comes first, unless a search is typed.
        let searching = !self.search.trim().is_empty();
        let first = hook.is_some() && !searching;
        if projection.listed() && projection.items().is_empty() && !first {
            self.arriving = None;
            let empty = Words::from(t::NO_ARTWORKS).shown(&ctx);
            list.centered_and_justified(|ui| ui.label(empty));
        } else if found.is_empty() && !projection.items().is_empty() && !first {
            let text = t::nothing_found(language, self.search.trim());
            list.centered_and_justified(|ui| ui.label(text));
        } else {
            let card = hook.as_deref_mut().filter(|_| !searching);
            egui::ScrollArea::vertical().show(&mut list, |ui| {
                self.grid(ui, &found, enabled, (card, &mut *taps));
            });
        }
        self.rename_dialog(ui.ctx(), taps);
        self.confirm_dialog(ui.ctx(), taps);
        self.options_dialog(ui.ctx(), projection.items(), taps);
        if let Some(hook) = hook {
            hook.dialogs(ui.ctx(), taps);
            self.opened_by(hook);
        }
        self.sizes_popover(ui.ctx(), under, taps);
    }

    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        items: &[&ProjectSummary],
        enabled: bool,
        (hook, taps): Taps<'_, '_>,
    ) {
        // Left-aligned rows, `CARD_GAP` around and between the cards.
        let pitch = card_size() + Vec2::splat(CARD_GAP);
        let width = ui.available_width();
        let columns = (((width - CARD_GAP) / pitch.x).floor() as usize).max(1);
        let first = usize::from(hook.is_some());
        let rows = (items.len() + first).div_ceil(columns);
        let origin = ui.cursor().min + Vec2::splat(CARD_GAP);
        let height = rows as f32 * pitch.y + CARD_GAP;
        ui.allocate_space(Vec2::new(width, height));
        let place = |slot: usize| {
            let cell = Vec2::new((slot % columns) as f32, (slot / columns) as f32);
            egui::Rect::from_min_size(origin + cell * pitch, card_size())
        };
        if let Some(hook) = hook {
            let sheet = hook.card(ui, place(0), enabled, taps);
            self.opened_by(hook);
            if let Some(arrival) = self.arriving.filter(|arrival| arrival.card)
                && land(ui.ctx(), &arrival, None, sheet)
            {
                self.arriving = None;
            }
        }
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
            let rect = place(index + first);
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

    /// Closes an open dialog as Cancel would (a mode's too), or the new-canvas popover. Returns
    /// whether one was open.
    pub(crate) fn back(&mut self, hook: Option<&mut (dyn GalleryHook + '_)>) -> bool {
        let mode = hook.is_some_and(|hook| hook.back());
        let open = self.rename.is_some()
            || self.confirm.is_some()
            || self.options.is_some()
            || mode
            || self.sizes != Sizes::Closed;
        self.rename = None;
        self.confirm = None;
        self.options = None;
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
            let name = format!("thumbnail-{}", item.id.get());
            let picture = (thumbnail.size(), thumbnail.pixels(), Alpha::Straight);
            let texture = picture_texture(ctx, name, picture);
            self.pictures.insert(item.id, (thumbnail.clone(), texture));
        }
    }
}

/// `title` as the pass's language shows it: the app's "Untitled Artwork" in the language, the
/// user's as typed. What the search looks in, and what a restored version's title is noted on.
pub(crate) fn shown_title(ctx: &egui::Context, title: &Title) -> Title {
    Title::new(&t::shown_name(ctx, title.as_str())).unwrap_or_else(|_| title.clone())
}
