//! Words the hand taps (one ring rule): a word in ink, bare on the paper, one touch target
//! high and at least one wide (the frame was never the hit area). Pressed, a faint wash fills
//! the pill the word would carry if chosen, nothing wider; unavailable, the word fades to pencil
//! grey and taps go nowhere. A command stays bare. A choice among words (one of a few options,
//! such as the selection's shape) belongs to a `choices` group, whose one ink pill (the icon's
//! ring drawn round the word) slides to the word chosen. When a bar holds a choice and
//! commands, a `divider` sets them apart; never a frame inside a frame. The words show in the
//! pass's language; the control goes by their English (`text.rs`).

use egui::{Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::components::{Choices, choices, controls};
use crate::design::{
    INK, INK_ACTIVE, INK_DISABLED, MARK_LINE, Outline, SPACE, TOUCH_TARGET, paint_divider,
    paint_pressed,
};
use crate::text::Words;

/// Shows a command reading `words`; only an `enabled` one can be clicked.
pub fn text_button<'a>(
    ui: &mut egui::Ui,
    words: impl Into<Words<'a>>,
    enabled: bool,
) -> egui::Response {
    button(ui, words.into(), (enabled, None), TOUCH_TARGET, false)
}

/// A page's heading that is also a command (a brush's name, which renames it): the words
/// in the heading's size, bare in ink, the press wash round them.
pub fn heading_button<'a>(ui: &mut egui::Ui, words: impl Into<Words<'a>>) -> egui::Response {
    let words = words.into();
    ui.scope(|ui| {
        let style = ui.style_mut();
        let heading = style.text_styles.get(&TextStyle::Heading).cloned();
        if let Some(font) = heading {
            style.text_styles.insert(TextStyle::Button, font);
        }
        button(ui, words, (true, None), TOUCH_TARGET, false)
    })
    .inner
}

/// A word reading `words` in the group `group`, showing whether it is the one `chosen`.
pub fn text_choice<'a>(
    ui: &mut egui::Ui,
    group: &mut Choices,
    words: impl Into<Words<'a>>,
    chosen: bool,
) -> egui::Response {
    button(
        ui,
        words.into(),
        (true, Some((group, chosen))),
        TOUCH_TARGET,
        false,
    )
}

/// The one thing to do on a page (Start, One more): a command whose word stands in its ink pill;
/// unavailable, word and pill fade to pencil grey.
pub fn pill_button<'a>(
    ui: &mut egui::Ui,
    words: impl Into<Words<'a>>,
    enabled: bool,
) -> egui::Response {
    button(ui, words.into(), (enabled, None), TOUCH_TARGET, true)
}

/// As `text_choice`, at least `width` wide (options in even columns); unavailable ones grey.
pub fn text_choice_wide<'a>(
    ui: &mut egui::Ui,
    group: &mut Choices,
    words: impl Into<Words<'a>>,
    (enabled, chosen): (bool, bool),
    width: f32,
) -> egui::Response {
    button(
        ui,
        words.into(),
        (enabled, Some((group, chosen))),
        width,
        false,
    )
}

/// What lies left of two columns of words.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lead<'a> {
    /// Nothing: the columns take the whole width.
    None,
    /// The panel's two spaces in.
    Indent,
    /// Two spaces in, then the words in a column `width` points wide on the first row.
    Label(Words<'a>, f32),
}

/// The group `salt` of `words` (each its words, whether it is chosen and whether it may be) in
/// two even columns after `lead`, the chosen one in the ink pill (one group: the pill slides
/// across the columns). Returns the index of a word tapped that was not chosen.
pub fn word_columns(
    ui: &mut egui::Ui,
    salt: &str,
    words: &[(Words<'_>, bool, bool)],
    lead: Lead<'_>,
) -> Option<usize> {
    let (indent, label) = match lead {
        Lead::None => (0.0, None),
        Lead::Indent => (2.0 * SPACE, None),
        Lead::Label(text, width) => (2.0 * SPACE, Some((text, width))),
    };
    let used = indent + label.map_or(0.0, |(_, width)| width + SPACE);
    let half = ((ui.available_width() - used) / 2.0).floor();
    let mut tapped = None;
    choices(ui, salt, |ui, group| {
        for (row, pair) in words.chunks(2).enumerate() {
            ui.horizontal(|ui| {
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                if let Some((text, width)) = label {
                    lead_label(ui, (row == 0).then_some(text), width);
                }
                for (column, &(words, on, enabled)) in pair.iter().enumerate() {
                    let word = text_choice_wide(ui, group, words, (enabled, on), half);
                    if word.clicked() && !on {
                        tapped = Some(2 * row + column);
                    }
                }
            });
        }
    });
    tapped
}

/// A column `width` points wide and a touch target high left of a row of words, `text` in ink
/// at its left.
fn lead_label(ui: &mut egui::Ui, text: Option<Words<'_>>, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), Sense::hover());
    if let Some(text) = text {
        let font = TextStyle::Body.resolve(ui.style());
        let at = rect.left_center();
        let shown = text.shown(ui.ctx());
        ui.painter()
            .text(at, egui::Align2::LEFT_CENTER, shown, font, INK);
    }
}

/// A thin upright line between a bar's choice and its commands; it takes no width of its own,
/// lying in the middle of the usual gap between the two.
pub fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(0.0, TOUCH_TARGET), Sense::hover());
    paint_divider(
        ui.painter(),
        rect.left(),
        rect.center().y,
        TOUCH_TARGET / 2.0,
    );
}

/// `choice` is `None` for a command, its group and whether it is chosen for a choice; a `pilled`
/// command stands in its pill.
fn button(
    ui: &mut egui::Ui,
    words: Words<'_>,
    (enabled, choice): (bool, Option<(&mut Choices, bool)>),
    min_width: f32,
    pilled: bool,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let font = TextStyle::Button.resolve(ui.style());
    let on = choice.as_ref().is_some_and(|(_, chosen)| *chosen);
    let color = match (enabled, on) {
        (false, _) => INK_DISABLED,
        (true, true) => INK_ACTIVE,
        (true, false) => INK,
    };
    let shown = words.shown(ui.ctx());
    let galley = ui.painter().layout_no_wrap(shown.to_owned(), font, color);
    // A pilled word stands well inside its pill.
    let pad = if pilled { 7.0 * SPACE } else { 4.0 * SPACE };
    let width = (galley.size().x + pad).max(min_width);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, on, shown));
    controls::register(ui, words.id(), rect);
    // The pill holds the word, not the whole column of a wide choice.
    let hug = Vec2::new(width.min(galley.size().x + pad), TOUCH_TARGET);
    let outline = Outline::pill(Rect::from_center_size(rect.center(), hug));
    if let Some((group, true)) = choice {
        group.chosen(outline);
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.is_pointer_button_down_on() {
            paint_pressed(painter, outline);
        }
        if pilled {
            let line = egui::Stroke::new(MARK_LINE, color);
            let kind = egui::StrokeKind::Middle;
            painter.rect_stroke(outline.rect, outline.corner, line, kind);
        }
        let text = Rect::from_center_size(rect.center(), galley.size());
        painter.galley(text.min, galley, color);
    }
    response
}
