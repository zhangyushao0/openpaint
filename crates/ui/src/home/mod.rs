//! The home (S122): two entrances side by side, a mode's column at the left (`HomeHook::column`;
//! not while a search is typed) and the studio at the right: "Studio" over every artwork whose
//! title holds the search's text as a card in a grid, the one changed last first (Procreate's
//! gallery). The top band holds the search field in its middle and, at the right, an island with
//! the "+" that makes a new artwork of a size chosen in a small popover or opens an OpenPaint file
//! (`new.rs`); a notice lies in a pill under the band at the right, under "+", so however long it
//! never covers the search field (S158h). Tap a picture to open it, its title to rename it (a
//! dialog under the islands, above the soft keyboard), the options to save it as an OpenPaint file
//! or restore a kept version (`dialogs.rs`, `dates.rs`), the copy button to duplicate it and the
//! bin to delete it, which asks first: it cannot be undone. A mode's whole pages lie over the home
//! with an island of their own, its dialogs after the home's own; a mode's sheet opened from its
//! page goes back into it (`HomeHook::landing`, S158h).

mod dates;
mod dialogs;
mod new;
mod studio;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use egui::{Align2, Rect, Vec2};
use openpaint_app::{
    GalleryIntent, GalleryProjection, Intent, MAX_TITLE_CHARS, ProjectId, ProjectSummary,
    Thumbnail, Title,
};

use crate::components::carry::{self, Arrival, Opening};
use crate::components::{
    Alpha, ISLAND_TOP, Naming, Place, Source, TOP_BAND, UNDER_ISLANDS, fitted, icon_button,
    icon_toggle, island, notice, picture_texture, pill, text_field,
};
use crate::design::{CARD_GAP, DESK, Icon, SPACE};
use crate::hooks::{HomeHook, Page};
use crate::text as t;

/// A mode's part of the home this pass, and the home's taps.
type Taps<'a, 'h> = (Option<&'a mut (dyn HomeHook + 'h)>, &'a mut Vec<Intent>);

/// The island "+" sits on; the new-canvas popover grows out of it.
const ISLAND: &str = "gallery island";
/// The search field's width in points.
const SEARCH_WIDTH: f32 = 360.0;
/// A mode's column's width at most, in points, and its share of the screen's at most.
const COLUMN: f32 = 600.0;
const COLUMN_SHARE: f32 = 0.42;

/// The new-canvas popover.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Sizes {
    #[default]
    Closed,
    /// Opened by this frame's tap, which must not close it again.
    Opened,
    Shown,
}

/// The home's UI-local state.
#[derive(Default)]
pub(crate) struct HomeScreen {
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

impl std::fmt::Debug for HomeScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HomeScreen")
            .field("sizes", &self.sizes)
            .field("pictures", &self.pictures.len())
            .field("rename", &self.rename)
            .field("confirm", &self.confirm)
            .field("options", &self.options)
            .field("search", &self.search)
            .finish()
    }
}

impl HomeScreen {
    /// The artwork just opened or made, once: the canvas grows out of where it lay.
    pub(crate) fn take_opening(&mut self) -> Option<Opening> {
        self.opening.take()
    }

    /// The canvas, last shown at `from` (points), comes back into the card of `id` (or the
    /// first card), a mode's sheet into the mode's column with the `picture` it left with, or
    /// `into` that place on the mode's page it was opened from (S158h).
    pub(crate) fn arrive(
        &mut self,
        from: Rect,
        (id, card, into): (Option<ProjectId>, bool, Option<Rect>),
        picture: Option<egui::TextureHandle>,
    ) {
        self.arrivals += 1;
        self.arriving = Some(Arrival {
            from,
            id,
            card,
            run: self.arrivals,
            picture,
            into,
        });
    }

    /// What the mode opened (if anything) since it was last asked: the canvas grows out of it.
    fn opened_by(&mut self, hook: &mut (dyn HomeHook + '_)) {
        if let Some(opening) = hook.opening() {
            self.opening = Some(opening);
        }
    }

    /// The whole screen inside `ui`: a mode's page first; unless one shows, the top band; unless
    /// one covers them, the two entrances on the desk. Returns where the screen lies.
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        projection: &GalleryProjection,
        (mut hook, taps): Taps<'_, '_>,
    ) -> Rect {
        let ctx = ui.ctx().clone();
        let page = self.page(&ctx, (hook.as_deref_mut(), &mut *taps));
        if page == Page::None {
            // No page to go back into: the column takes the sheet.
            if let Some(arrival) = self.arriving.as_mut() {
                arrival.into = None;
            }
            self.islands(&ctx, projection, (hook.as_deref_mut(), &mut *taps));
        } else {
            self.land_on_page(&ctx, hook.as_deref_mut());
        }
        // On the desk, so the list's edge fade has its colour.
        let desk = egui::CentralPanel::default().frame(egui::Frame::new().fill(DESK));
        let shown = desk.show(ui, |ui| {
            if page != Page::Covers {
                self.body(ui, projection, (hook, taps));
            }
            ui.max_rect()
        });
        shown.inner
    }

    /// A mode's page over the home, drawn with its own island; how far it lies over it.
    fn page(&mut self, ctx: &egui::Context, (hook, taps): Taps<'_, '_>) -> Page {
        let Some(hook) = hook else {
            return Page::None;
        };
        let page = hook.page(ctx, taps);
        self.opened_by(hook);
        page
    }

    /// The top band: the search field in the middle, and at the right an island with "+" and,
    /// under it, a pill with any notice (S158h: in the band, a long one lay over the search
    /// field). "+" opens the new-canvas popover (and closes it again).
    pub(crate) fn islands(
        &mut self,
        ctx: &egui::Context,
        projection: &GalleryProjection,
        (hook, taps): Taps<'_, '_>,
    ) {
        egui::Area::new(egui::Id::new("gallery search"))
            .fade_in(false)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, ISLAND_TOP))
            .show(ctx, |ui| {
                ui.set_width(SEARCH_WIDTH);
                text_field(ui, t::SEARCH, &mut self.search, MAX_TITLE_CHARS, false);
            });
        let enabled = projection.availability().is_available();
        let open = self.sizes != Sizes::Closed;
        let (button, _) = island(ctx, ISLAND, Place::top_right(0.0), |ui| {
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
        // The home's own notice first, then the mode's.
        let told = (t::gallery_notice(projection.notice()).map(|m| (m, true)))
            .or_else(|| Some((hook.as_deref()?.notice()?, false)));
        if let Some((message, own)) = told {
            // Under the band, its right end under the island's: the search field stays whole.
            let place = Place {
                anchor: Align2::RIGHT_TOP,
                offset: Vec2::new(-SPACE, UNDER_ISLANDS),
            };
            let source = Source::Within(egui::Align::Max);
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

    /// Under the top band inside `ui`: the mode's column at the left (unless a search is typed),
    /// the studio beside it, and an open dialog.
    pub(crate) fn body(
        &mut self,
        ui: &mut egui::Ui,
        projection: &GalleryProjection,
        (mut hook, taps): Taps<'_, '_>,
    ) {
        let ctx = ui.ctx().clone();
        self.load_pictures(&ctx, projection.items());
        // While the sizes are open a tap elsewhere only closes them.
        let enabled = projection.availability().is_available() && self.sizes == Sizes::Closed;
        let mut under = ui.max_rect();
        under.min.y += TOP_BAND;
        let searching = !self.search.trim().is_empty();
        let mut studio = under;
        if let Some(hook) = hook.as_deref_mut().filter(|_| !searching) {
            let width = COLUMN.min(under.width() * COLUMN_SHARE);
            let column = Rect::from_min_size(
                under.min + Vec2::new(CARD_GAP, SPACE),
                Vec2::new(width, under.height() - SPACE - CARD_GAP),
            );
            studio.min.x = column.right();
            let place = hook.column(ui, column, enabled, taps);
            self.opened_by(hook);
            self.land_sheet(&ctx, hook, place);
        }
        self.studio(ui, studio, (projection, enabled), taps);
        self.rename_dialog(ui.ctx(), taps);
        self.confirm_dialog(ui.ctx(), taps);
        self.options_dialog(ui.ctx(), projection.items(), taps);
        if let Some(hook) = hook {
            hook.dialogs(ui.ctx(), taps);
            self.opened_by(hook);
        }
        self.sizes_popover(ui.ctx(), under, taps);
    }

    /// A mode's sheet coming back lands in `place`, the mode's (`HomeHook::column`), at its own
    /// shape: the picture it left with and the mode's marks over it shrink there together, as an
    /// artwork's picture into its card (S144).
    fn land_sheet(&mut self, ctx: &egui::Context, hook: &mut (dyn HomeHook + '_), place: Rect) {
        let arrival = self.arriving.as_ref();
        let Some(arrival) = arrival.filter(|arrival| arrival.card && arrival.into.is_none()) else {
            return;
        };
        let into = fitted(place, arrival.from.size());
        let (at, landed) = carry::carried(ctx, arrival, into);
        let painter = carry::sheet_at(ctx, arrival.picture.as_ref(), at);
        hook.carried(&painter, (arrival.from, at));
        if landed {
            self.arriving = None;
        }
    }

    /// A mode's sheet going back into the page it was opened from (S158h) lands in its place
    /// there, over the page, at its own shape.
    fn land_on_page(&mut self, ctx: &egui::Context, hook: Option<&mut (dyn HomeHook + '_)>) {
        let Some((arrival, place)) = (self.arriving.as_ref())
            .filter(|arrival| arrival.card)
            .and_then(|arrival| Some((arrival, arrival.into?)))
        else {
            return;
        };
        let into = fitted(place, arrival.from.size());
        let (at, landed) = carry::carried(ctx, arrival, into);
        let painter = carry::sheet_over(ctx, arrival.picture.as_ref(), at);
        if let Some(hook) = hook {
            hook.carried(&painter, (arrival.from, at));
        }
        if landed {
            self.arriving = None;
        }
    }

    /// Closes an open dialog as Cancel would (a mode's too), or the new-canvas popover. Returns
    /// whether one was open.
    pub(crate) fn back(&mut self, hook: Option<&mut (dyn HomeHook + '_)>) -> bool {
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
