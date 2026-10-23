//! The practice options, grown out of the Today card (S80): the stages one can move to (the next
//! one before its time, an earlier one to revisit), each starting a session at once; the
//! practice packs kept, each with how many pictures it holds and how many of them have an
//! outline, the user's own as My pictures; Import practice pack… and Add my picture…, each
//! opening the system's picker; the long colour copies (S82), each begun one to open and a new
//! one of a pack's picture or of the user's own. Close or a tap outside closes them.

use egui::{Align, Layout};
use openpaint_app::{CopyFrom, GalleryIntent, Intent, Language, TodayProjection};

use super::GalleryScreen;
use crate::components::{Placement, card_text, dialog, menu_row, section, text_button};
use crate::design::SPACE;
use crate::text::{self as t, Words};

/// The practice packs kept, Import practice pack… and Add my picture…; returns what was tapped.
fn packs(ui: &mut egui::Ui, today: &TodayProjection) -> Option<GalleryIntent> {
    let language = t::language(ui.ctx());
    let mut chosen = None;
    section(ui, t::PRACTICE_PACKS);
    if today.packs.is_empty() {
        note(ui, t::NO_PACKS.into(), true);
    }
    for pack in &today.packs {
        let title = if pack.personal {
            t::MY_PICTURES.get(language).to_owned()
        } else {
            pack.title.clone()
        };
        let line = t::pack_line(language, &title, (pack.pictures, pack.outlines));
        note(ui, Words::made(&line), false);
    }
    if menu_row(ui, t::IMPORT_PACK, Words::EMPTY, true).clicked() {
        chosen = Some(GalleryIntent::ImportPack);
    }
    if menu_row(ui, t::ADD_PICTURE, Words::EMPTY, true).clicked() {
        chosen = Some(GalleryIntent::AddPicture);
    }
    note(ui, t::PACKS_NOTE.into(), true);
    chosen
}

/// The long colour copies: each begun, to open, and a new one of a pack's picture or of the
/// user's own (where there are such); returns what was tapped.
fn copies(ui: &mut egui::Ui, today: &TodayProjection) -> Option<GalleryIntent> {
    let language = t::language(ui.ctx());
    let mut chosen = None;
    section(ui, t::COLOUR_COPIES);
    for copy in &today.copies {
        let title = copy.title.as_deref();
        let name = t::copy_row(language, copy.number, title);
        let english = t::copy_row(Language::English, copy.number, title);
        if menu_row(ui, Words::named(&english, &name), t::OPEN, true).clicked() {
            chosen = Some(GalleryIntent::Open(copy.sheet));
        }
    }
    let [pack, mine] = today.copy_from;
    if pack && menu_row(ui, t::COPY_A_PACK_PICTURE, Words::EMPTY, true).clicked() {
        chosen = Some(GalleryIntent::Copy(CopyFrom::Pack));
    }
    if mine && menu_row(ui, t::COPY_MY_PICTURE, Words::EMPTY, true).clicked() {
        chosen = Some(GalleryIntent::Copy(CopyFrom::Mine));
    }
    note(ui, t::COPIES_NOTE.into(), true);
    chosen
}

/// A paragraph of the options, wrapped, set in as far as the rows' words.
fn note(ui: &mut egui::Ui, text: Words<'_>, quiet: bool) {
    let inset = egui::Margin {
        left: (2.0 * SPACE) as i8,
        ..egui::Margin::ZERO
    };
    egui::Frame::NONE
        .inner_margin(inset)
        .show(ui, |ui| card_text(ui, text, quiet));
}

impl GalleryScreen {
    /// The practice options while they show.
    pub(super) fn practice_dialog(
        &mut self,
        ctx: &egui::Context,
        today: &TodayProjection,
        taps: &mut Vec<Intent>,
    ) {
        let Some(card) = self.practice_options else {
            return;
        };
        let language = t::language(ctx);
        let ((chosen, close), outside) = dialog(ctx, "practice", (Placement::Top, card), |ui| {
            ui.heading(Words::from(t::PRACTICE_OPTIONS).shown(ui.ctx()));
            let mut chosen = None;
            section(ui, t::STAGES);
            for stage in (1..=today.ready).filter(|stage| *stage != today.stage) {
                let name = t::stage(language, stage);
                let english = t::stage(openpaint_app::Language::English, stage);
                if menu_row(ui, Words::named(&english, &name), t::START, true).clicked() {
                    chosen = Some(GalleryIntent::Practice(stage));
                }
            }
            note(ui, t::STAGE_HINT.into(), true);
            chosen = packs(ui, today).or(chosen);
            chosen = copies(ui, today).or(chosen);
            ui.add_space(SPACE);
            let close = ui
                .with_layout(Layout::right_to_left(Align::Center), |ui| {
                    text_button(ui, t::CLOSE, true).clicked()
                })
                .inner;
            (chosen, close)
        });
        if let Some(intent) = chosen {
            let sheet = matches!(
                intent,
                GalleryIntent::Practice(_) | GalleryIntent::Copy(_) | GalleryIntent::Open(_)
            );
            if sheet {
                self.opening = Some(crate::components::carry::Opening {
                    from: card,
                    id: None,
                    picture: None,
                });
            }
            taps.push(Intent::Gallery(intent));
            self.practice_options = None;
        } else if close || outside {
            self.practice_options = None;
        }
    }
}
