//! The dialog: a panel over a dimmed screen that must be answered before anything else. It is
//! used only to confirm what cannot be undone and to type (ARCHITECTURE section 10). It grows
//! out of what was tapped as the veil darkens, and goes back into it once answered (D44). A tap
//! on the dimmed screen around it cancels.

use egui::{Align, Align2, Id, Layout, Margin, Order, Rect, Shape, Vec2};

use crate::components::chrome::{self, Source};
use crate::components::{UNDER_ISLANDS, hint, text_button, text_field};
use crate::design::{BACKDROP, CORNER, DIALOG_WIDTH, PANEL_RULE_WIDTH, SPACE, surface_frame};

/// Where a dialog sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// In the middle of the screen.
    Centre,
    /// Under the top islands, so the soft keyboard rising from the bottom never covers it.
    Top,
}

/// Shows the dialog `id`, grown out of `from` (points), with `add_contents` laid out top to
/// bottom, `DIALOG_WIDTH` wide. Returns what the contents returned and whether the user tapped
/// outside, which cancels.
pub fn dialog<T>(
    ctx: &egui::Context,
    id: &str,
    (placement, from): (Placement, Rect),
    add_contents: impl FnOnce(&mut egui::Ui) -> T,
) -> (T, bool) {
    let (align, offset) = match placement {
        Placement::Centre => (Align2::CENTER_CENTER, Vec2::ZERO),
        Placement::Top => (Align2::CENTER_TOP, Vec2::new(0.0, UNDER_ISLANDS)),
    };
    let key = Id::new(id);
    let source = Source::Rect {
        rect: from,
        radius: (0.0, CORNER),
        veil: true,
    };
    let showing = chrome::declare(ctx, key, source, Order::Foreground, true);
    let (reveal, grown) = showing.map_or((1.0, 1.0), |s| (s.reveal, s.grown));
    let area = egui::Modal::default_area(key)
        .order(Order::Foreground)
        // Whole in the frame it opens: feedback within one frame.
        .fade_in(false)
        .anchor(align, offset);
    let margin = 2.0 * SPACE;
    let response = egui::Modal::new(key)
        .area(area)
        .frame(surface_frame(Margin::same(margin as i8)))
        .backdrop_color(BACKDROP.gamma_multiply(grown))
        .show(ctx, |ui| {
            let paper = ui.painter().add(Shape::Noop);
            ui.multiply_opacity(reveal);
            ui.set_width(DIALOG_WIDTH - 4.0 * SPACE);
            ui.spacing_mut().item_spacing = Vec2::splat(SPACE);
            let inner = add_contents(ui);
            (
                inner,
                paper,
                ui.min_rect().expand(margin + PANEL_RULE_WIDTH),
            )
        });
    let (inner, paper, rect) = response.inner;
    let layer = response.response.layer_id;
    chrome::laid_out(ctx, key, rect, layer, Some(paper));
    let outside = response.backdrop_response.clicked();
    (inner, outside)
}

/// A name being typed in a naming dialog: the text so far, whether the field takes the focus in
/// the next pass, and what was tapped (points): the dialog grows from there.
#[derive(Debug)]
pub struct Naming {
    pub text: String,
    focus: bool,
    from: Rect,
}

impl Naming {
    /// `text` to edit, all of it selected, for a dialog growing from `from`.
    #[must_use]
    pub fn new(text: &str, from: Rect) -> Self {
        Self {
            text: text.to_owned(),
            focus: true,
            from,
        }
    }
}

/// The dialog `id` under the islands naming something: `heading`, the text field `field` (at most
/// `max_chars`) holding `naming.text`, then Cancel and Done, grey unless `valid` takes the text.
/// Returns `Some(true)` once Done (or Enter on a valid text) answered, `Some(false)` once Cancel
/// or a tap outside did, `None` while it waits.
pub fn naming_dialog(
    ctx: &egui::Context,
    id: &str,
    (heading, field, max_chars): (&str, &str, usize),
    naming: &mut Naming,
    valid: impl Fn(&str) -> bool,
) -> Option<bool> {
    let ((cancel, done), outside) = dialog(ctx, id, (Placement::Top, naming.from), |ui| {
        ui.heading(heading);
        let typed = text_field(ui, field, &mut naming.text, max_chars, naming.focus);
        if typed.response.has_focus() {
            naming.focus = false;
        }
        let valid = valid(&naming.text);
        let (cancel, done) = answer_row(ui, "Cancel", "Done", valid);
        (cancel, done || (typed.submitted && valid))
    });
    if done {
        Some(true)
    } else if cancel || outside {
        Some(false)
    } else {
        None
    }
}

/// The dialog `id` in the middle of the screen, grown from `from` (points), asking before
/// something is deleted for good: `question` as its heading, `note` in the quieter grey under it,
/// then Cancel and Delete. Returns `Some(true)` once Delete answered, `Some(false)` once Cancel
/// or a tap outside did, `None` while it waits.
pub fn delete_dialog(
    ctx: &egui::Context,
    id: &str,
    from: Rect,
    (question, note): (&str, &str),
) -> Option<bool> {
    let ((cancel, delete), outside) = dialog(ctx, id, (Placement::Centre, from), |ui| {
        ui.heading(question);
        hint(ui, note);
        answer_row(ui, "Cancel", "Delete", true)
    });
    if delete {
        Some(true)
    } else if cancel || outside {
        Some(false)
    } else {
        None
    }
}

/// Two words at the right of a dialog's last row: the cancelling one, then the answering one
/// (grey unless `enabled`). Returns whether each was tapped.
pub fn answer_row(ui: &mut egui::Ui, cancel: &str, answer: &str, enabled: bool) -> (bool, bool) {
    ui.add_space(SPACE);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let answered = text_button(ui, answer, enabled).clicked();
        let cancelled = text_button(ui, cancel, true).clicked();
        (cancelled, answered)
    })
    .inner
}
