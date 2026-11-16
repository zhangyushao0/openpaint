//! The projects in the practice options (S83, S151): the long colour copies with New copy…, and
//! the drawings from life once there are some, each a row with its picture and name; a project's
//! page (its picture larger with its title and credit, or that it was removed; Open, Rename…,
//! Remove…), renaming it (its sheet's title, the gallery's naming dialog) and removing it or a
//! picture of one's own (the gallery's delete dialog, asking first and saying what goes and what
//! stays). Answered, the options show again.

use egui::{Rect, Vec2};
use openpaint_app::{GalleryIntent, MAX_TITLE_CHARS, ProjectId, Title};
use openpaint_practice::Tier;
use openpaint_practice_app::{CopyShown, OptionsIntent, PathProjection};

use openpaint_ui::components::{
    ListRow, delete_dialog, fitted, list_row, menu_row, naming_dialog, paint_picture, section,
};
use openpaint_ui::design::Icon;

use super::HomePart;
use super::options::{Page, Removal, Shelf, Tapped, note, row_picture};
use super::pictures::OptionsPictures;
use crate::text::{self as t, Words};

/// A copy page's picture, in points.
const PAGE_PICTURE: Vec2 = Vec2::new(200.0, 150.0);

/// The page of the colour copy whose sheet is `sheet`.
pub(super) fn copy_page(
    ui: &mut egui::Ui,
    sheet: ProjectId,
    path: &PathProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let Some(copy) = path.copies.iter().find(|copy| copy.sheet == sheet) else {
        // Gone (removed meanwhile): back to the first page.
        return Some(Tapped::Page(Page::Main));
    };
    let name = t::shown_name(ui.ctx(), copy.name.as_str()).into_owned();
    ui.heading(&name);
    let size = Vec2::new(ui.available_width(), PAGE_PICTURE.y);
    let (place, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let frame = Rect::from_center_size(place.center(), PAGE_PICTURE);
    let picture = copy.picture.as_ref().and_then(|item| path.picture(item));
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
            let title = t::shown_name(ui.ctx(), &picture.title).into_owned();
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

impl HomePart {
    /// The practice options' naming or removing dialog, whichever waits; answered, the options
    /// show again (the copy renamed, the first page after a copy went, My pictures after one of
    /// them went).
    pub(super) fn answer_dialogs(&mut self, ctx: &egui::Context, taps: &mut Vec<OptionsIntent>) {
        let card = self.practice.card().unwrap_or(Rect::NOTHING);
        if let Some((sheet, naming)) = &mut self.practice.rename {
            let words = (t::RENAME, t::NAME, MAX_TITLE_CHARS);
            let valid = |text: &str| Title::new(text).is_ok();
            let Some(done) = naming_dialog(ctx, "rename copy", words, naming, valid) else {
                return;
            };
            let sheet = *sheet;
            if done && let Ok(title) = Title::new(&naming.text) {
                taps.push(OptionsIntent::RenameCopy { sheet, title });
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
                OptionsIntent::RemoveCopy(*sheet),
                t::COPY_GOES.get(language).to_owned(),
                Page::Main,
            ),
            Removal::Picture(item, name, uses) => (
                name.as_str(),
                OptionsIntent::RemovePicture(item.clone()),
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
            taps.push(intent);
        }
        self.practice.remove = None;
        self.practice.show(after);
    }
}
/// The long colour copies, each with its picture and name (tapped, it opens; its options button
/// shows its page), and New copy… while there are pictures; returns what was tapped.
pub(super) fn copies(
    ui: &mut egui::Ui,
    path: &PathProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let mut chosen = None;
    section(ui, t::COLOUR_COPIES);
    for copy in path.copies.iter().filter(|copy| !copy.from_life) {
        chosen = copy_row(ui, copy, path, pictures).or(chosen);
    }
    let any = !path.pictures.is_empty();
    if menu_row(ui, t::NEW_COPY, Words::EMPTY, any).clicked() {
        let first =
            (Tier::ALL.into_iter()).find(|tier| path.pictures_of(Some(*tier)).next().is_some());
        chosen = Some(Tapped::Page(Page::Picker(
            first.map_or(Shelf::Mine, Shelf::Tier),
        )));
    }
    chosen
}

/// The drawings from life (S151), once there are some, each as a colour copy's row; returns what
/// was tapped.
pub(super) fn drawings(
    ui: &mut egui::Ui,
    path: &PathProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let mut drawn = path.copies.iter().filter(|copy| copy.from_life).peekable();
    drawn.peek()?;
    section(ui, t::LIFE_DRAWINGS);
    let mut chosen = None;
    for copy in drawn {
        chosen = copy_row(ui, copy, path, pictures).or(chosen);
    }
    chosen
}

/// A project's row: its picture and name (tapped, it opens), its options button (its page).
fn copy_row(
    ui: &mut egui::Ui,
    copy: &CopyShown,
    path: &PathProjection,
    pictures: &mut OptionsPictures,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let shown = t::shown_name(ui.ctx(), copy.name.as_str());
    let picture = copy.picture.as_ref().and_then(|item| path.picture(item));
    // One's own picture's title in the language ("Picture 1", 图片 1).
    let detail = picture.map_or(t::PICTURE_REMOVED.get(language).into(), |p| {
        t::shown_name(ui.ctx(), &p.title)
    });
    let detail = detail.as_ref();
    let texture = (copy.picture.as_ref()).and_then(|item| pictures.texture(item));
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
        Some(Tapped::Page(Page::Copy(copy.sheet)))
    } else if row.row.clicked() {
        Some(Tapped::Intent(GalleryIntent::Open(copy.sheet), true))
    } else {
        None
    }
}
