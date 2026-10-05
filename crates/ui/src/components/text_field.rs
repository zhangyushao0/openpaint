//! The text field: one line of text, one touch target high, across its panel. Focused, it asks
//! the host for the keyboard (egui's IME output); Enter ends the edit.

use egui::{Color32, TextEdit, TextStyle, Vec2};

use crate::components::controls;
use crate::design::{INK_SECONDARY, SPACE, TOUCH_TARGET};

/// What the field did this frame.
pub struct FieldResponse {
    pub response: egui::Response,
    /// Enter was pressed: the edit is finished.
    pub submitted: bool,
}

/// Shows `text` for editing, at most `max_chars` characters, named `name` (also shown in the
/// quieter ink while it is empty). With `focus`, the field takes the focus with all its text
/// selected, so typing replaces it.
pub fn text_field(
    ui: &mut egui::Ui,
    name: &str,
    text: &mut String,
    max_chars: usize,
    focus: bool,
) -> FieldResponse {
    let id = egui::Id::new(("text field", name));
    let output = TextEdit::singleline(text)
        .id(id)
        .hint_text(egui::RichText::new(name).color(INK_SECONDARY))
        .font(TextStyle::Body)
        .char_limit(max_chars)
        .desired_width(f32::INFINITY)
        .min_size(Vec2::new(0.0, TOUCH_TARGET))
        .margin(Vec2::new(2.0 * SPACE, SPACE))
        .background_color(Color32::WHITE)
        .vertical_align(egui::Align::Center)
        .show(ui);
    let response = output.response.response;
    controls::register(ui, name, response.rect);
    if focus {
        response.request_focus();
        let mut state = output.state;
        let all = egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(text.chars().count()),
        );
        state.cursor.set_char_range(Some(all));
        state.store(ui.ctx(), id);
    }
    let submitted = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    FieldResponse {
        response,
        submitted,
    }
}
