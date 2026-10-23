//! The card: a piece of paper at the screen's right, under the right island, holding a few lines
//! to read (how to practise, a comparison, a session's summary) beside what they are about, or
//! the tools an exercise uses beside its frame. It grows out of what asked for it and goes back
//! into it (D44); it is not modal: the canvas left of it stays the hand's.

use egui::{Align2, Id, Margin, Order, Rect, RichText, Sense, TextStyle, Vec2};

use crate::components::chrome::Source;
use crate::components::paper::paper;
use crate::design::{DIALOG_WIDTH, INK, INK_SECONDARY, SPACE, surface_frame};
use crate::text::Words;

/// Width of a card's number column, in points.
const COLUMN: f32 = 96.0;

/// Shows the card `id`, one space in from the screen's right with its top `top` points from the
/// screen's (`UNDER_ISLANDS`, or under another card), grown out of `source`, with
/// `add_contents` laid out top to bottom, `DIALOG_WIDTH` wide. Returns what the contents returned
/// and the card's rectangle in points, which nothing under it may take a contact from.
pub fn card<R>(
    ctx: &egui::Context,
    id: &str,
    (source, top): (Source, f32),
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, Rect) {
    let area = egui::Area::new(Id::new(id))
        .order(Order::Middle)
        .movable(false)
        .constrain(false)
        .fade_in(false)
        .anchor(Align2::RIGHT_TOP, Vec2::new(-SPACE, top));
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
/// quieter ink. Chinese is set as Chinese is (`chinese_lines`).
pub fn card_text<'a>(ui: &mut egui::Ui, text: impl Into<Words<'a>>, quiet: bool) {
    let text = text.into().shown(ui.ctx());
    let style = if quiet {
        TextStyle::Small
    } else {
        TextStyle::Body
    };
    let text = if text.chars().any(is_han) {
        let font = style.resolve(ui.style());
        chinese_lines(ui.ctx(), text, &font, ui.available_width())
    } else {
        text.to_owned()
    };
    let text = if quiet {
        RichText::new(text).small().color(INK_SECONDARY)
    } else {
        RichText::new(text).color(INK)
    };
    ui.label(text);
}

/// Marks that never begin a line in Chinese typesetting (they stay with what comes before), and
/// those that never end one (they go with what follows).
const NO_LINE_START: &str = "，。、：；？！）》」』”’…%°,.:;?!)";
const NO_LINE_END: &str = "（《「『“‘(";

fn is_han(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c)
}

/// A letter or digit of a word set among Chinese ("ΔE", "Oklab", "12°"): kept whole on a line.
fn in_word(c: char) -> bool {
    !c.is_whitespace() && !is_han(c) && !NO_LINE_START.contains(c) && !NO_LINE_END.contains(c)
}

/// `text` with a line break wherever the next unit would pass `width` points in `font`: a unit
/// is a character with the marks that may not begin a line after it and those that may not end
/// one before it, or a whole Latin word or number. egui's own breaking would set a full-width
/// comma or colon at a line's start.
fn chinese_lines(ctx: &egui::Context, text: &str, font: &egui::FontId, width: f32) -> String {
    let mut units: Vec<String> = Vec::new();
    for c in text.chars() {
        // A mark after a space ("11 %") stays with what stands before the space.
        if NO_LINE_START.contains(c)
            && units.len() > 1
            && units.last().is_some_and(|unit| unit.trim().is_empty())
            && let Some(space) = units.pop()
            && let Some(before) = units.last_mut()
        {
            before.push_str(&space);
        }
        let joins = units
            .last()
            .and_then(|unit| unit.chars().last())
            .is_some_and(|last| {
                NO_LINE_START.contains(c)
                    || NO_LINE_END.contains(last)
                    || in_word(last) && in_word(c)
            });
        match units.last_mut() {
            Some(unit) if joins => unit.push(c),
            _ => units.push(c.to_string()),
        }
    }
    let mut lines = String::with_capacity(text.len() + 8);
    let mut used = 0.0;
    ctx.fonts_mut(|fonts| {
        for unit in &units {
            let wide: f32 = unit.chars().map(|c| fonts.glyph_width(font, c)).sum();
            if used > 0.0 && used + wide > width {
                lines.push('\n');
                used = 0.0;
                if unit.trim().is_empty() {
                    continue;
                }
            }
            lines.push_str(unit);
            used += wide;
        }
    });
    lines
}

/// A row of a card's table: the first cell at the left, each other in a column of its own at the
/// right, in ink, or `quiet` (column heads) in the quieter ink; the cells in the language shown.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_never_starts_a_line_with_a_comma_and_keeps_words_whole() {
        let ctx = egui::Context::default();
        crate::design::install(&ctx);
        let font = egui::FontId::proportional(18.0);
        let text = "先看再画：找出两个端点和弯得最厉害的地方，判断它们在框里的位置。ΔE：Oklab × 100，\
                    约 2 时刚能看出差别。你的颜色：更青 12°，更纯 12 %，更暗 11 %。";
        for width in [60.0, 97.0, 133.0, 180.0, 250.0] {
            // A frame's pass: fonts are ready to measure.
            let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
            output.textures_delta.clear();
            let lines = chinese_lines(&ctx, text, &font, width);
            let joined: String = lines.split('\n').collect();
            assert_eq!(joined.replace(' ', ""), text.replace(' ', ""), "{width}");
            for line in lines.split('\n').skip(1) {
                let first = line.chars().next().expect("no empty line");
                assert!(
                    !NO_LINE_START.contains(first) && !first.is_whitespace(),
                    "{width}: {line}"
                );
            }
            assert!(
                lines.split('\n').any(|line| line.contains("Oklab")),
                "{width}"
            );
        }
    }
}
