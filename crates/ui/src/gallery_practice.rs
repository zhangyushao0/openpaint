//! The practice options, grown out of the Today card (S80, S83), one dialog whose page changes
//! in place: the stages one can move to (the next one before its time, an earlier one to
//! revisit), each starting a session at once; the practice packs kept, each with how many
//! pictures it holds and how many of them have an outline, the user's own as My pictures (a page
//! of their own: `gallery_pictures.rs`); Import practice pack… and Add my picture… (a tier first,
//! then the system's picker); the long colour copies (S82), each with its picture and name, to
//! open, or on its page to rename or remove (`gallery_copies.rs`), and New copy…, a picker of the
//! packs' pictures by tier and the user's own. Removing asks first, as the gallery's delete does.
//! Back goes to the first page; Close or a tap outside closes them.

use egui::{Align, Layout, Rect};
use openpaint_app::{GalleryIntent, Intent, Language, ProjectId, TodayProjection};
use openpaint_practice::{ItemRef, Tier};

use super::GalleryScreen;
use super::pictures::OptionsPictures;
use crate::components::{
    ListRow, Naming, Placement, RowPicture, UNDER_ISLANDS, card_text, dialog, list_row, menu_row,
    section, text_button,
};
use crate::design::{Icon, LAYER_PICTURE, SPACE, TOUCH_TARGET};
use crate::text::{self as t, Words};

/// What a page holds above and below its column of rows (its heading, its last row and the
/// dialog's margins), in points: the column scrolls in the rest of the screen.
const CHROME_ROOM: f32 = 4.0 * TOUCH_TARGET;

/// Which page the practice options show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Page {
    Main,
    /// New copy's picker: the packs' pictures of a tier, or the user's own (`None`).
    Picker(Option<Tier>),
    /// My pictures, one of them, and adding one (its tier first).
    Mine,
    Picture(ItemRef),
    Add,
    /// A colour copy's page, by its sheet.
    Copy(ProjectId),
}

/// What waits for the user's answer before it goes for good.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Removal {
    /// A colour copy (its sheet) and its name as shown.
    Copy(ProjectId, String),
    /// A picture of one's own, its title as shown, and how many sessions and copies used it.
    Picture(ItemRef, String, (u32, u32)),
}

/// What a page's tap asks.
pub(super) enum Tapped {
    /// The gallery's intent, and whether it opens a sheet (grown out of the Today card).
    Intent(GalleryIntent, bool),
    Page(Page),
    /// A colour copy renamed: its sheet and its name as shown, in the naming dialog.
    Rename(ProjectId, String),
    Remove(Removal),
}

/// The practice options' state while the gallery shows.
#[derive(Default)]
pub(crate) struct PracticeOptions {
    /// Where they grew from (the Today card, points) and the page shown; none while closed.
    shown: Option<(Rect, Page)>,
    /// A colour copy being renamed (its sheet) and its name as typed.
    pub(super) rename: Option<(ProjectId, Naming)>,
    /// What waits for the user's answer before it is removed.
    pub(super) remove: Option<Removal>,
    /// The packs' pictures as the options show them small.
    pub(crate) pictures: OptionsPictures,
}

impl std::fmt::Debug for PracticeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PracticeOptions")
            .field("shown", &self.shown)
            .field("rename", &self.rename)
            .field("remove", &self.remove)
            .finish_non_exhaustive()
    }
}

impl PracticeOptions {
    /// Opens them on their first page, grown from `card`.
    pub(super) fn open(&mut self, card: Rect) {
        self.shown = Some((card, Page::Main));
    }

    /// Closes them and any dialog of theirs as Cancel would. Returns whether one was open.
    pub(super) fn close(&mut self) -> bool {
        let open = self.shown.is_some() || self.rename.is_some() || self.remove.is_some();
        (self.shown, self.rename, self.remove) = (None, None, None);
        open
    }

    /// Where they grew from, while they show.
    pub(super) fn card(&self) -> Option<Rect> {
        self.shown.as_ref().map(|(card, _)| *card)
    }

    /// They show `page` from now on, if they show.
    pub(super) fn show(&mut self, page: Page) {
        if let Some((_, shown)) = &mut self.shown {
            *shown = page;
        }
    }
}

impl GalleryScreen {
    /// The practice options while they show, or their rename or remove dialog over the gallery.
    pub(super) fn practice_dialog(
        &mut self,
        ctx: &egui::Context,
        today: &TodayProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.practice.pictures.upload(ctx);
        if self.practice.rename.is_some() || self.practice.remove.is_some() {
            self.answer_dialogs(ctx, taps);
            return;
        }
        let Some((card, page)) = self.practice.shown.clone() else {
            return;
        };
        let options = &mut self.practice;
        let ((tapped, back, close), outside) =
            dialog(ctx, "practice", (Placement::Top, card), |ui| {
                let tapped = page_of(ui, &page, today, &mut options.pictures);
                let (back, close) = bottom_row(ui, page != Page::Main);
                (tapped, back, close)
            });
        match tapped {
            Some(Tapped::Intent(intent, opens)) => {
                if opens {
                    self.opening = Some(crate::components::carry::Opening {
                        from: card,
                        id: None,
                        picture: None,
                    });
                }
                let stays = matches!(intent, GalleryIntent::PictureTier(..));
                taps.push(Intent::Gallery(intent));
                if !stays {
                    self.practice.shown = None;
                }
            }
            Some(Tapped::Page(page)) => self.practice.shown = Some((card, page)),
            Some(Tapped::Rename(sheet, name)) => {
                self.practice.rename = Some((sheet, Naming::new(&name, card)));
            }
            Some(Tapped::Remove(removal)) => self.practice.remove = Some(removal),
            None if back => self.practice.shown = Some((card, Page::Main)),
            None if close || outside => self.practice.shown = None,
            None => {}
        }
    }
}

/// Back at the left of the last row (but on the first page) and Close at the right; whether each
/// was tapped.
fn bottom_row(ui: &mut egui::Ui, back: bool) -> (bool, bool) {
    ui.add_space(SPACE);
    ui.horizontal(|ui| {
        let went = back && text_button(ui, t::BACK, true).clicked();
        let closed = ui
            .with_layout(Layout::right_to_left(Align::Center), |ui| {
                text_button(ui, t::CLOSE, true).clicked()
            })
            .inner;
        (went, closed)
    })
    .inner
}

/// The page `page` laid out; what was tapped.
fn page_of(
    ui: &mut egui::Ui,
    page: &Page,
    today: &TodayProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    pictures.begin();
    let room = ui.ctx().content_rect().height() - UNDER_ISLANDS - CHROME_ROOM;
    match page {
        Page::Main => {
            ui.heading(Words::from(t::PRACTICE_OPTIONS).shown(ui.ctx()));
            scrolled(ui, room, |ui| main_page(ui, today, pictures))
        }
        Page::Picker(tier) => super::pictures::picker(ui, (*tier, room), today, pictures),
        Page::Mine => super::pictures::mine(ui, room, today, pictures),
        Page::Picture(item) => super::pictures::picture(ui, item, today, pictures),
        Page::Add => super::pictures::add(ui),
        Page::Copy(sheet) => super::copies::copy_page(ui, *sheet, today, pictures),
    }
}

/// `add` in a column that scrolls once it is taller than `room` points.
pub(super) fn scrolled<R>(ui: &mut egui::Ui, room: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::ScrollArea::vertical()
        .max_height(room.max(4.0 * TOUCH_TARGET))
        // A finger drags the column (egui asks for a touch screen otherwise).
        .scroll_source(egui::scroll_area::ScrollSource::ALL)
        .show(ui, add)
        .inner
}

/// The first page: the stages, the packs and the colour copies.
fn main_page(
    ui: &mut egui::Ui,
    today: &TodayProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let mut chosen = None;
    section(ui, t::STAGES);
    for stage in (1..=today.ready).filter(|stage| *stage != today.stage) {
        let name = t::stage(language, stage);
        let english = t::stage(Language::English, stage);
        let row = menu_row(ui, Words::named(&english, &name), t::START, true);
        if row.clicked() {
            chosen = Some(Tapped::Intent(GalleryIntent::Practice(stage), true));
        }
    }
    note(ui, t::STAGE_HINT.into(), true);
    chosen = packs(ui, today).or(chosen);
    chosen = copies(ui, today, pictures).or(chosen);
    chosen
}

/// The practice packs kept, My pictures, Import practice pack… and Add my picture…; returns what
/// was tapped.
fn packs(ui: &mut egui::Ui, today: &TodayProjection) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let mut chosen = None;
    section(ui, t::PRACTICE_PACKS);
    if today.packs.is_empty() {
        note(ui, t::NO_PACKS.into(), true);
    }
    for pack in &today.packs {
        if pack.personal {
            let count = t::pictures_count(language, pack.pictures);
            if menu_row(ui, t::MY_PICTURES, Words::made(&count), true).clicked() {
                chosen = Some(Tapped::Page(Page::Mine));
            }
        } else {
            let line = t::pack_line(language, &pack.title, (pack.pictures, pack.outlines));
            note(ui, Words::made(&line), false);
        }
    }
    if menu_row(ui, t::IMPORT_PACK, Words::EMPTY, true).clicked() {
        chosen = Some(Tapped::Intent(GalleryIntent::ImportPack, false));
    }
    if menu_row(ui, t::ADD_PICTURE, Words::EMPTY, true).clicked() {
        chosen = Some(Tapped::Page(Page::Add));
    }
    note(ui, t::PACKS_NOTE.into(), true);
    chosen
}

/// The long colour copies, each with its picture and name (tapped, it opens; its options button
/// shows its page), and New copy… while there are pictures; returns what was tapped.
fn copies(
    ui: &mut egui::Ui,
    today: &TodayProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let mut chosen = None;
    section(ui, t::COLOUR_COPIES);
    for copy in &today.copies {
        let shown = t::shown_name(language, copy.name.as_str());
        let picture = copy.picture.as_ref().and_then(|item| today.picture(item));
        let detail = picture.map_or(t::PICTURE_REMOVED.get(language), |p| p.title.as_str());
        let texture = copy
            .picture
            .as_ref()
            .and_then(|item| pictures.texture(item));
        let row = list_row(
            ui,
            &ListRow {
                key: egui::Id::new(("colour copy", copy.sheet.get())),
                label: Words::named(copy.name.as_str(), &shown),
                detail,
                indent: 0.0,
                lead: None,
                picture: Some(row_picture(texture)),
                selected: false,
                icon: Icon::More,
                icon_label: t::COPY_OPTIONS,
                enabled: true,
                lifted: None,
            },
        );
        if let Some(item) = &copy.picture {
            pictures.want(ui, item, row.row.rect);
        }
        if row.icon.clicked() {
            chosen = Some(Tapped::Page(Page::Copy(copy.sheet)));
        } else if row.row.clicked() {
            let open = GalleryIntent::Open(copy.sheet);
            chosen = Some(Tapped::Intent(open, true));
        }
    }
    let any = !today.pictures.is_empty();
    if menu_row(ui, t::NEW_COPY, Words::EMPTY, any).clicked() {
        let first =
            (Tier::ALL.into_iter()).find(|tier| today.pictures_of(Some(*tier)).next().is_some());
        chosen = Some(Tapped::Page(Page::Picker(first)));
    }
    note(ui, t::COPIES_NOTE.into(), true);
    chosen
}

/// A list row's picture: `texture` (its id and size) fitted in a layer row's place, or the empty
/// place while it is read.
pub(super) fn row_picture(texture: Option<(egui::TextureId, egui::Vec2)>) -> RowPicture {
    let place = Rect::from_min_size(egui::Pos2::ZERO, LAYER_PICTURE);
    RowPicture {
        texture: texture.map(|(id, _)| id),
        size: texture.map_or(LAYER_PICTURE, |(_, size)| {
            crate::components::fitted(place, size).size()
        }),
    }
}

/// A paragraph of the options, wrapped, set in as far as the rows' words.
pub(super) fn note(ui: &mut egui::Ui, text: Words<'_>, quiet: bool) {
    let inset = egui::Margin {
        left: (2.0 * SPACE) as i8,
        ..egui::Margin::ZERO
    };
    egui::Frame::NONE
        .inner_margin(inset)
        .show(ui, |ui| card_text(ui, text, quiet));
}
