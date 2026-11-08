//! The Today card, first in the gallery (G9): a sheet like an artwork's picture with what to
//! practise written on it (the stage and its topic, a hint where shapes wait for a pack), the
//! stage's road (S89: its sessions as dots, those done inked, the next ringed) and the one thing
//! to do, Start practice, Continue or Start the next stage once a stage's sessions are done, as a
//! bare word in ink at its foot; under it, like an artwork's title, "Practice" and the options
//! button (stages, practice packs, the user's own pictures). The rest of the sheet opens the
//! practice book; pressed, it darkens under the hand.

use egui::{Align, Align2, Color32, Layout, Rect, Sense, TextStyle, UiBuilder, Vec2};
use openpaint_ui::components::{card_size, controls, icon_button, paint_sheet, text_button};
use openpaint_ui::design::{
    CARD_PICTURE, INK, INK_DISABLED, INK_SECONDARY, Icon, SPACE, TOUCH_TARGET,
};

use crate::components::paint_stretch;
use crate::text::{PRACTICE, PRACTICE_BOOK, PRACTICE_OPTIONS, TODAY, Text, Words};

/// What the Today card shows.
pub struct TodayCard<'a> {
    /// "Stage 1 · Lines and colours", in the language shown.
    pub stage: &'a str,
    /// The stage's sessions done and about how many it holds: its road.
    pub road: (u32, u32),
    /// A quieter line under them, if any.
    pub hint: Option<Text>,
    /// Start practice, Continue practice or Start the next stage.
    pub action: Text,
    pub enabled: bool,
}

/// What was tapped on the Today card, and where its sheet lies (points).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TodayTaps {
    /// The sheet: the practice book.
    pub book: bool,
    pub action: bool,
    pub options: bool,
    pub rect: Rect,
}

/// Shows the card.
pub fn today_card(ui: &mut egui::Ui, card: &TodayCard<'_>) -> TodayTaps {
    let (rect, _) = ui.allocate_exact_size(card_size(), Sense::hover());
    let sheet = Rect::from_min_size(rect.min, CARD_PICTURE);
    let sense = if card.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(sheet, ui.id().with("today"), sense);
    controls::register(ui, PRACTICE_BOOK.id(), sheet);
    let tint = if response.is_pointer_button_down_on() {
        Color32::from_gray(200)
    } else {
        Color32::WHITE
    };
    paint_sheet(ui.painter(), None, sheet, tint);
    let row = Rect::from_min_max(sheet.left_bottom(), rect.max);
    paint_words(ui, card, (sheet, row));
    // The one thing to do, at the sheet's foot: over the sheet, so it takes its own taps.
    let foot = Rect::from_min_size(
        sheet.left_bottom() - Vec2::new(-SPACE, TOUCH_TARGET + SPACE / 2.0),
        Vec2::new(sheet.width() - 2.0 * SPACE, TOUCH_TARGET),
    );
    let mut words = ui.new_child(
        UiBuilder::new()
            .id_salt("today action")
            .max_rect(foot)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let action = text_button(&mut words, card.action, card.enabled).clicked();
    let mut buttons = ui.new_child(
        UiBuilder::new()
            .id_salt("today options")
            .max_rect(row)
            .layout(Layout::right_to_left(Align::Center)),
    );
    buttons.spacing_mut().item_spacing = Vec2::ZERO;
    let options = icon_button(&mut buttons, Icon::More, PRACTICE_OPTIONS, card.enabled);
    TodayTaps {
        book: response.clicked() && !action,
        action,
        options: options.clicked(),
        rect: sheet,
    }
}

/// The card's words and road on its `sheet` and, under it in `row`, "Practice".
fn paint_words(ui: &egui::Ui, card: &TodayCard<'_>, (sheet, row): (Rect, Rect)) {
    let words = |text: Text| Words::from(text).shown(ui.ctx());
    let painter = ui.painter();
    let font = |style: TextStyle| style.resolve(ui.style());
    let left = sheet.left() + 2.0 * SPACE;
    let mut y = sheet.top() + 2.0 * SPACE;
    let heading = font(TextStyle::Heading);
    painter.text(
        egui::pos2(left, y),
        Align2::LEFT_TOP,
        words(TODAY),
        heading,
        INK,
    );
    y += 4.0 * SPACE;
    let hint = card.hint.map(words);
    for line in [Some(card.stage), hint].into_iter().flatten() {
        let body = font(TextStyle::Body);
        painter.text(
            egui::pos2(left, y),
            Align2::LEFT_TOP,
            line,
            body,
            INK_SECONDARY,
        );
        y += 3.0 * SPACE;
    }
    let road = Rect::from_min_max(
        egui::pos2(sheet.left() + SPACE, sheet.bottom() - 13.0 * SPACE),
        egui::pos2(sheet.right() - SPACE, sheet.bottom() - 7.0 * SPACE),
    );
    paint_stretch(painter, road, card.road, None);
    let ink = if card.enabled { INK } else { INK_DISABLED };
    let at = egui::pos2(row.left(), row.center().y);
    let body = font(TextStyle::Body);
    painter.text(at, Align2::LEFT_CENTER, words(PRACTICE), body, ink);
}
