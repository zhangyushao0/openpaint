//! The Today card, first in the gallery (G9): a sheet like an artwork's picture with what to
//! practise written on it (the stage and its topic, the sessions finished of about how many, a
//! hint where shapes wait for a pack) and the one thing to do, Start practice, Continue or Start
//! the next stage once a stage's sessions are done, as a bare word in ink; under it, like an
//! artwork's title, "Practice" and the options button (stages, practice packs, the user's own
//! pictures). The whole sheet is the button. Pressed, it darkens under the hand.

use egui::{Align, Align2, Color32, Layout, Rect, Sense, TextStyle, UiBuilder, Vec2};

use crate::components::{card_size, controls, icon_button, paint_sheet};
use crate::design::{CARD_PICTURE, INK, INK_DISABLED, INK_SECONDARY, Icon, SPACE};
use crate::text::{PRACTICE, PRACTICE_OPTIONS, TODAY, Text, Words};

/// What the Today card shows.
pub struct TodayCard<'a> {
    /// "Stage 1 · Lines and colours", in the language shown.
    pub stage: &'a str,
    /// "3 of about 15 sessions done", in the language shown.
    pub progress: &'a str,
    /// A quieter line under them, if any.
    pub hint: Option<Text>,
    /// Start practice, Continue practice or Start the next stage: the sheet's name as a control
    /// too.
    pub action: Text,
    pub enabled: bool,
}

/// What was tapped on the Today card, and where its sheet lies (points).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TodayTaps {
    pub sheet: bool,
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
    controls::register(ui, card.action.id(), sheet);
    let tint = if response.is_pointer_button_down_on() {
        Color32::from_gray(200)
    } else {
        Color32::WHITE
    };
    paint_sheet(ui.painter(), None, sheet, tint);
    let row = Rect::from_min_max(sheet.left_bottom(), rect.max);
    paint_words(ui, card, (sheet, row));
    let mut buttons = ui.new_child(
        UiBuilder::new()
            .id_salt("today options")
            .max_rect(row)
            .layout(Layout::right_to_left(Align::Center)),
    );
    buttons.spacing_mut().item_spacing = Vec2::ZERO;
    let options = icon_button(&mut buttons, Icon::More, PRACTICE_OPTIONS, card.enabled);
    TodayTaps {
        sheet: response.clicked(),
        options: options.clicked(),
        rect: sheet,
    }
}

/// The card's words on its `sheet` and, under it in `row`, "Practice".
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
    for line in [Some(card.stage), Some(card.progress), hint]
        .into_iter()
        .flatten()
    {
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
    let ink = if card.enabled { INK } else { INK_DISABLED };
    let at = egui::pos2(left, sheet.bottom() - 2.0 * SPACE);
    let button = font(TextStyle::Button);
    painter.text(at, Align2::LEFT_BOTTOM, words(card.action), button, ink);
    let at = egui::pos2(row.left(), row.center().y);
    let body = font(TextStyle::Body);
    painter.text(at, Align2::LEFT_CENTER, words(PRACTICE), body, ink);
}
