//! The card: a piece of paper at the screen's right, under the right island, holding a few lines
//! to read (how to practise, a check's numbers, a session's summary) beside what they are about.
//! It grows out of the word that asked for it and goes back into it (D44); it is not modal: the
//! canvas left of it stays the hand's.

use egui::{Align2, Id, Margin, Order, Rect, RichText, Sense, TextStyle, Vec2};

use crate::components::UNDER_ISLANDS;
use crate::components::chrome::Source;
use crate::components::paper::paper;
use crate::design::{DIALOG_WIDTH, INK, INK_SECONDARY, SPACE, surface_frame};

/// Width of a card's number column, in points.
const COLUMN: f32 = 96.0;

/// Shows the card `id`, one space in from the screen's right and under the islands, grown out of
/// `source`, with `add_contents` laid out top to bottom, `DIALOG_WIDTH` wide. Returns what the
/// contents returned and the card's rectangle in points, which nothing under it may take a
/// contact from.
pub fn card<R>(
    ctx: &egui::Context,
    id: &str,
    source: Source,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, Rect) {
    let area = egui::Area::new(Id::new(id))
        .order(Order::Middle)
        .movable(false)
        .constrain(false)
        .fade_in(false)
        .anchor(Align2::RIGHT_TOP, Vec2::new(-SPACE, UNDER_ISLANDS));
    let margin = 2.0 * SPACE;
    paper(ctx, Id::new(id), source, area, |ui| {
        surface_frame(Margin::same(margin as i8))
            .show(ui, |ui| {
                ui.set_width(DIALOG_WIDTH - 2.0 * margin);
                ui.spacing_mut().item_spacing = Vec2::new(SPACE, SPACE / 2.0);
                add_contents(ui)
            })
            .inner
    })
}

/// A paragraph of a card, wrapped to its width: in ink, or `quiet` (a note) smaller and in the
/// quieter ink.
pub fn card_text(ui: &mut egui::Ui, text: &str, quiet: bool) {
    let text = if quiet {
        RichText::new(text).small().color(INK_SECONDARY)
    } else {
        RichText::new(text).color(INK)
    };
    ui.label(text);
}

/// A row of a card's table: the first cell at the left, each other in a column of its own at the
/// right, in ink, or `quiet` (column heads) in the quieter ink.
pub fn card_columns(ui: &mut egui::Ui, cells: &[&str], quiet: bool) {
    let font = TextStyle::Body.resolve(ui.style());
    let height = ui.text_style_height(&TextStyle::Body) + SPACE / 2.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let colour = if quiet { INK_SECONDARY } else { INK };
    let painter = ui.painter();
    for (index, cell) in cells.iter().enumerate() {
        if index == 0 {
            painter.text(
                rect.left_center(),
                Align2::LEFT_CENTER,
                cell,
                font.clone(),
                colour,
            );
        } else {
            let right = rect.right() - (cells.len() - 1 - index) as f32 * COLUMN;
            let at = egui::pos2(right, rect.center().y);
            painter.text(at, Align2::RIGHT_CENTER, cell, font.clone(), colour);
        }
    }
}
