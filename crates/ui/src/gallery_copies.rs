//! A long colour copy in the practice options (S83): its page (its picture larger with its title
//! and credit, or that it was removed; Open, Rename…, Remove…), renaming it (its sheet's title,
//! the gallery's naming dialog) and removing it or a picture of one's own (the gallery's delete
//! dialog, asking first and saying what goes and what stays). Answered, the options show again.

use egui::{Rect, Vec2};
use openpaint_app::{GalleryIntent, Intent, MAX_TITLE_CHARS, ProjectId, Title, TodayProjection};

use super::GalleryScreen;
use super::pictures::OptionsPictures;
use super::practice::{Page, Removal, Tapped, note};
use crate::components::{delete_dialog, fitted, menu_row, naming_dialog, paint_picture};
use crate::text::{self as t, Words};

/// A copy page's picture, in points.
const PAGE_PICTURE: Vec2 = Vec2::new(200.0, 150.0);

/// The page of the colour copy whose sheet is `sheet`.
pub(super) fn copy_page(
    ui: &mut egui::Ui,
    sheet: ProjectId,
    today: &TodayProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let Some(copy) = today.copies.iter().find(|copy| copy.sheet == sheet) else {
        // Gone (removed meanwhile): back to the first page.
        return Some(Tapped::Page(Page::Main));
    };
    let name = t::shown_name(language, copy.name.as_str()).into_owned();
    ui.heading(&name);
    let size = Vec2::new(ui.available_width(), PAGE_PICTURE.y);
    let (place, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let frame = Rect::from_center_size(place.center(), PAGE_PICTURE);
    let picture = copy.picture.as_ref().and_then(|item| today.picture(item));
    let texture = copy
        .picture
        .as_ref()
        .and_then(|item| pictures.texture(item));
    if let Some(item) = &copy.picture {
        pictures.want(ui, item, place);
    }
    let shown = texture.map_or(frame, |(_, size)| fitted(frame, size));
    paint_picture(ui.painter(), texture.map(|(id, _)| id), shown);
    match picture {
        Some(picture) => {
            let title = t::shown_name(language, &picture.title).into_owned();
            note(ui, Words::made(&title), false);
            if let Some(credit) = &picture.credit {
                note(ui, Words::made(credit), true);
            }
        }
        None => note(ui, t::PICTURE_REMOVED.into(), true),
    }
    let mut tapped = None;
    if menu_row(ui, t::OPEN, Words::EMPTY, true).clicked() {
        tapped = Some(Tapped::Intent(GalleryIntent::Open(sheet), true));
    }
    if menu_row(ui, t::RENAME_COPY, Words::EMPTY, true).clicked() {
        tapped = Some(Tapped::Rename(sheet, name.clone()));
    }
    if menu_row(ui, t::REMOVE, Words::EMPTY, true).clicked() {
        tapped = Some(Tapped::Remove(Removal::Copy(sheet, name)));
    }
    tapped
}

impl GalleryScreen {
    /// The practice options' naming or removing dialog, whichever waits; answered, the options
    /// show again (the copy renamed, the first page after a copy went, My pictures after one of
    /// them went).
    pub(super) fn answer_dialogs(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let card = self.practice.card().unwrap_or(Rect::NOTHING);
        if let Some((sheet, naming)) = &mut self.practice.rename {
            let words = (t::RENAME, t::NAME, MAX_TITLE_CHARS);
            let valid = |text: &str| Title::new(text).is_ok();
            let Some(done) = naming_dialog(ctx, "rename copy", words, naming, valid) else {
                return;
            };
            let sheet = *sheet;
            if done && let Ok(title) = Title::new(&naming.text) {
                taps.push(Intent::Gallery(GalleryIntent::RenameCopy { sheet, title }));
            }
            self.practice.rename = None;
            self.practice.show(Page::Copy(sheet));
            return;
        }
        let Some(removal) = &self.practice.remove else {
            return;
        };
        let language = t::language(ctx);
        let (name, intent, goes, after) = match removal {
            Removal::Copy(sheet, name) => (
                name.as_str(),
                GalleryIntent::RemoveCopy(*sheet),
                t::COPY_GOES.get(language).to_owned(),
                Page::Main,
            ),
            Removal::Picture(item, name, uses) => (
                name.as_str(),
                GalleryIntent::RemovePicture(item.clone()),
                t::picture_goes(language, *uses),
                Page::Mine,
            ),
        };
        let question = t::remove_named(language, name);
        let note = |ui: &mut egui::Ui| {
            ui.label(&goes);
        };
        let Some(remove) = delete_dialog(ctx, "remove practice", card, &question, note) else {
            return;
        };
        if remove {
            taps.push(Intent::Gallery(intent));
        }
        self.practice.remove = None;
        self.practice.show(after);
    }
}
