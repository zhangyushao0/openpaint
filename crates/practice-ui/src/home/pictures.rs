//! The practice options' pictures (S83): New copy's picker (the packs' pictures of a tier, or the
//! user's own, each with its credit: tapped, a long colour copy of it begins), My pictures (each
//! with its tier and whether it has an outline; tapped, its page: its tier chosen again, Remove…
//! asking first and saying what the sessions and copies that used it keep), and adding one (its
//! tier first, then the system's picker). Their small pictures come from the runtime's worker as
//! the rows show (`OptionsPictures`), kept for the app's run.

use std::collections::{HashMap, HashSet};

use egui::{Rect, TextureHandle, Vec2};

use openpaint_practice::{ItemRef, Tier};
use openpaint_practice_app::{OptionsIntent, PathProjection, PictureImage, PictureShown};

use openpaint_ui::components::{
    Alpha, Lead, ListRow, PictureRow, choices, fitted, label_row, list_row, menu_row,
    paint_picture, picture_row, picture_texture, text_choice, word_columns,
};
use openpaint_ui::design::{Icon, SPACE};

use super::options::{Page, Removal, Tapped, note, row_picture, scrolled};
use crate::text::{self as t, Words};

/// A picture page's picture, in points.
const PAGE_PICTURE: Vec2 = Vec2::new(200.0, 150.0);

/// The pictures the options show small: uploaded as the runtime reads them, asked for as their
/// rows show.
#[derive(Default)]
pub(crate) struct OptionsPictures {
    /// Each picture read, as a texture (none: it could not be read).
    shown: HashMap<ItemRef, Option<TextureHandle>>,
    /// Read and not uploaded yet.
    arrived: Vec<(ItemRef, Option<PictureImage>)>,
    /// This pass's rows' pictures not shown yet, the first shown first, and those asked for.
    wanted: Vec<ItemRef>,
    asked: HashSet<ItemRef>,
}

impl OptionsPictures {
    /// Uploads the pictures read since the last pass.
    pub(super) fn upload(&mut self, ctx: &egui::Context) {
        for (item, picture) in self.arrived.drain(..) {
            let texture = picture.map(|picture| {
                let name = format!("option-{}-{}", item.pack, item.item);
                picture_texture(ctx, name, (picture.size, &picture.pixels, Alpha::Straight))
            });
            self.shown.insert(item, texture);
        }
    }

    /// A pass begins: no row has asked yet.
    pub(super) fn begin(&mut self) {
        self.wanted.clear();
    }

    /// `item`'s picture, its texture and size, once read.
    pub(super) fn texture(&self, item: &ItemRef) -> Option<(egui::TextureId, Vec2)> {
        let texture = self.shown.get(item)?.as_ref()?;
        Some((texture.id(), texture.size_vec2()))
    }

    /// A row of `item` lies at `rect`: while it shows and its picture is not read, it is wanted.
    pub(super) fn want(&mut self, ui: &egui::Ui, item: &ItemRef, rect: Rect) {
        if ui.is_rect_visible(rect) && !self.shown.contains_key(item) {
            self.wanted.push(item.clone());
        }
    }

    /// The next picture to read: the first wanted not asked for yet.
    pub(crate) fn next_wanted(&mut self) -> Option<ItemRef> {
        let item = self
            .wanted
            .iter()
            .find(|item| !self.asked.contains(*item))?
            .clone();
        self.asked.insert(item.clone());
        Some(item)
    }

    /// `item`'s picture was read (none: it could not be): its row shows it from the next pass.
    pub(crate) fn arrive(&mut self, item: ItemRef, picture: Option<PictureImage>) {
        self.arrived.push((item, picture));
    }
}

/// A picture's title as the options show it (one's own "Picture n" in the language).
fn title(ctx: &egui::Context, picture: &PictureShown) -> String {
    t::shown_name(ctx, &picture.title).into_owned()
}

/// New copy's picker: the tiers that hold pictures and My pictures to choose from, then the
/// pictures of the one chosen, `room` points of them showing at once.
pub(super) fn picker(
    ui: &mut egui::Ui,
    (tier, room): (Option<Tier>, f32),
    path: &PathProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    ui.heading(Words::from(t::CHOOSE_PICTURE).shown(ui.ctx()));
    let tabs: Vec<Option<Tier>> = (Tier::ALL.into_iter().map(Some))
        .chain([None])
        .filter(|tab| path.pictures_of(*tab).next().is_some())
        .collect();
    let words: Vec<(Words<'_>, bool, bool)> = (tabs.iter())
        .map(|tab| {
            (
                tab.map_or(t::MY_PICTURES, t::tier).into(),
                *tab == tier,
                true,
            )
        })
        .collect();
    let mut chosen = word_columns(ui, "picture tiers", &words, Lead::None)
        .map(|at| Tapped::Page(Page::Picker(tabs[at])));
    note(ui, t::CHOOSE_HINT.into(), true);
    let rows = scrolled(ui, room - 3.0 * openpaint_ui::design::TOUCH_TARGET, |ui| {
        let mut tapped = None;
        let shown: Vec<&PictureShown> = path.pictures_of(tier).collect();
        if shown.is_empty() {
            note(ui, t::NO_PICTURES_HERE.into(), true);
        }
        for picture in shown {
            let name = title(ui.ctx(), picture);
            // One's own shows its tier (the tabs say the official ones'), then whether copied.
            let tier = (picture.personal).then(|| t::tier(picture.tier).get(language));
            let copied = (picture.copies > 0).then(|| t::COPIED_BEFORE.get(language));
            let note = [tier, copied]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
            let row = picture_row(
                ui,
                &PictureRow {
                    label: Words::named(&picture.title, &name),
                    note: &note,
                    credit: picture.credit.as_deref(),
                    picture: pictures.texture(&picture.item),
                    enabled: true,
                },
            );
            pictures.want(ui, &picture.item, row.rect);
            if row.clicked() {
                tapped = Some(Tapped::Options(
                    OptionsIntent::Copy(picture.item.clone()),
                    true,
                ));
            }
        }
        tapped
    });
    chosen = rows.or(chosen);
    chosen
}

/// My pictures: each with its tier and whether it has an outline (tapped, its page; its bin
/// asks before it removes it), Add my picture… and what becomes of them.
pub(super) fn mine(
    ui: &mut egui::Ui,
    room: f32,
    path: &PathProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    ui.heading(Words::from(t::MY_PICTURES).shown(ui.ctx()));
    scrolled(ui, room, |ui| {
        let mut tapped = None;
        let own: Vec<&PictureShown> = path.pictures_of(None).collect();
        if own.is_empty() {
            note(ui, t::NO_PICTURE_OF_MINE.into(), true);
        }
        for picture in own {
            let name = title(ui.ctx(), picture);
            let line = t::mine_line(
                language,
                t::tier(picture.tier).get(language),
                picture.outline,
            );
            let texture = pictures.texture(&picture.item);
            let row = list_row(
                ui,
                &ListRow {
                    key: egui::Id::new(("my picture", &picture.item.item)),
                    label: Words::named(&picture.title, &name),
                    detail: &line,
                    indent: 0.0,
                    lead: None,
                    picture: Some(row_picture(texture)),
                    selected: false,
                    icon: Icon::Delete,
                    icon_label: t::REMOVE_PICTURE,
                    enabled: true,
                    lifted: None,
                },
            );
            pictures.want(ui, &picture.item, row.row.rect);
            if row.icon.clicked() {
                tapped = Some(removal(ui.ctx(), picture));
            } else if row.row.clicked() {
                tapped = Some(Tapped::Page(Page::Picture(picture.item.clone())));
            }
        }
        if menu_row(ui, t::ADD_PICTURE, Words::EMPTY, true).clicked() {
            tapped = Some(Tapped::Page(Page::Add));
        }
        note(ui, t::MINE_NOTE.into(), true);
        tapped
    })
}

/// Removing `picture` asks first, saying what the sessions and copies that used it keep.
fn removal(ctx: &egui::Context, picture: &PictureShown) -> Tapped {
    let uses = (picture.sessions, picture.copies);
    Tapped::Remove(Removal::Picture(
        picture.item.clone(),
        title(ctx, picture),
        uses,
    ))
}

/// A picture of one's own: shown larger, its tier chosen again (at once), whether it has an
/// outline, and Remove….
pub(super) fn picture(
    ui: &mut egui::Ui,
    item: &ItemRef,
    path: &PathProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let Some(picture) = path.picture(item) else {
        // Gone (removed meanwhile): back to the list.
        return Some(Tapped::Page(Page::Mine));
    };
    ui.heading(title(ui.ctx(), picture));
    let (place, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), PAGE_PICTURE.y),
        egui::Sense::hover(),
    );
    let frame = Rect::from_center_size(place.center(), PAGE_PICTURE);
    let texture = pictures.texture(item);
    pictures.want(ui, item, place);
    let shown = texture.map_or(frame, |(_, size)| fitted(frame, size));
    paint_picture(ui.painter(), texture.map(|(id, _)| id), shown);
    let mut tapped = None;
    label_row(ui, t::HOW_HARD, true);
    ui.horizontal(|ui| {
        ui.add_space(SPACE);
        choices(ui, "picture tier", |ui, group| {
            for tier in Tier::ALL {
                let on = picture.tier == tier;
                if text_choice(ui, group, t::tier(tier), on).clicked() && !on {
                    let moved = OptionsIntent::PictureTier(item.clone(), tier);
                    tapped = Some(Tapped::Options(moved, false));
                }
            }
        });
    });
    let outline = if picture.outline {
        t::WITH_OUTLINE
    } else {
        t::WITHOUT_OUTLINE
    };
    note(ui, outline.into(), true);
    if menu_row(ui, t::REMOVE, Words::EMPTY, true).clicked() {
        tapped = Some(removal(ui.ctx(), picture));
    }
    tapped
}

/// Adding a picture of one's own: how hard it is to copy first, each tier with what it holds;
/// a tier tapped opens the system's picker.
pub(super) fn add(ui: &mut egui::Ui) -> Option<Tapped> {
    ui.heading(Words::from(t::ADD_MY_PICTURE).shown(ui.ctx()));
    note(ui, t::TIER_FIRST.into(), true);
    let mut tapped = None;
    for tier in Tier::ALL {
        if menu_row(ui, t::tier(tier), t::tier_hint(tier), true).clicked() {
            tapped = Some(Tapped::Options(OptionsIntent::AddPicture(tier), false));
        }
    }
    tapped
}
