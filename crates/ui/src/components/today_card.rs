//! The Today card, first in the gallery (G9): a sheet like an artwork's picture with what to
//! practise written on it (the stage and its topic, the sessions finished of about how many) and
//! the one thing to do, Start practice or Continue, as a bare word in ink; under it, like an
//! artwork's title, "Practice". The whole sheet is the button. Pressed, it darkens under the
//! hand.

use egui::{Align2, Color32, Rect, Sense, TextStyle};

use crate::components::{card_size, controls, paint_sheet};
use crate::design::{CARD_PICTURE, INK, INK_DISABLED, INK_SECONDARY, SPACE};

/// What the Today card shows.
pub struct TodayCard<'a> {
    /// "Stage 1 · Lines and colours".
    pub stage: &'a str,
    /// "3 of about 15 sessions done".
    pub progress: &'a str,
    /// "Start practice" or "Continue practice": the sheet's name as a control too.
    pub action: &'a str,
    pub enabled: bool,
}

/// Shows the card; returns whether its sheet was tapped, and where the sheet lies (points).
pub fn today_card(ui: &mut egui::Ui, card: &TodayCard<'_>) -> (bool, Rect) {
    let (rect, _) = ui.allocate_exact_size(card_size(), Sense::hover());
    let sheet = Rect::from_min_size(rect.min, CARD_PICTURE);
    let sense = if card.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(sheet, ui.id().with("today"), sense);
    controls::register(ui, card.action, sheet);
    let tint = if response.is_pointer_button_down_on() {
        Color32::from_gray(200)
    } else {
        Color32::WHITE
    };
    let painter = ui.painter();
    paint_sheet(painter, None, sheet, tint);
    let font = |style: TextStyle| style.resolve(ui.style());
    let left = sheet.left() + 2.0 * SPACE;
    let mut y = sheet.top() + 2.0 * SPACE;
    painter.text(
        egui::pos2(left, y),
        Align2::LEFT_TOP,
        "Today",
        font(TextStyle::Heading),
        INK,
    );
    y += 4.0 * SPACE;
    for line in [card.stage, card.progress] {
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
    painter.text(
        at,
        Align2::LEFT_BOTTOM,
        card.action,
        font(TextStyle::Button),
        ink,
    );
    let title = Rect::from_min_max(sheet.left_bottom(), rect.max);
    let at = egui::pos2(title.left(), title.center().y);
    painter.text(
        at,
        Align2::LEFT_CENTER,
        "Practice",
        font(TextStyle::Body),
        ink,
    );
    (response.clicked(), sheet)
}
