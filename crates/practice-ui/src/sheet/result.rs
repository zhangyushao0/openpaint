//! An exercise's result on the compared card (S122): Accurate, Almost and Again in a row, each
//! after its mark (a dot, a ring half filled, an empty ring). Where the app judges, the one it
//! found is ringed in ink and the comparison's items under it say why; where the eye judges, a
//! short prompt, then the three as words to tap (`PracticeIntent::Judge`), the one chosen ringed
//! from then on, and a quiet line on what Again does. Nothing while no verdict is honest yet; a
//! scribble has none.

use egui::{Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};
use openpaint_practice::{Outcome, Verdict};
use openpaint_practice_app::{Judged, PracticeIntent};

use openpaint_ui::components::{card_text, controls};
use openpaint_ui::design::{
    INK, INK_ACTIVE, INK_SECONDARY, Outline, SPACE, TOUCH_TARGET, paint_mark, paint_pressed,
};
use openpaint_ui::taps::push_once;

use crate::components::paint_result;
use crate::text::{self as t, Words};

/// The three results, in order.
const OUTCOMES: [Outcome; 3] = [Outcome::Clean, Outcome::Near, Outcome::Redo];
/// A result's mark's radius, in points.
const MARK: f32 = 7.0;

/// The result row for `judged`, its taps (the eye's choice) to `taps`.
pub(crate) fn result_row(ui: &mut egui::Ui, judged: Judged, taps: &mut Vec<PracticeIntent>) {
    let kept = judged.result.map(|(outcome, _)| outcome);
    let shown = match (judged.verdict, kept) {
        (Verdict::Incomplete, _) => return,
        (Verdict::App(outcome), _) => Some(outcome),
        (Verdict::Eye, kept) => kept,
    };
    let choosing = judged.verdict == Verdict::Eye && kept.is_none();
    if choosing {
        card_text(ui, t::YOUR_EYE, false);
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(SPACE / 2.0, 0.0);
        for outcome in OUTCOMES {
            let ringed = shown == Some(outcome);
            if outcome_word(ui, outcome, (choosing, ringed)).clicked() && choosing {
                push_once(taps, PracticeIntent::Judge(outcome));
            }
        }
    });
    if judged.verdict == Verdict::Eye {
        card_text(ui, t::EYE_NOTE, true);
    }
}

/// `outcome`'s word after its mark, one touch target high: a word to tap while `tappable`,
/// `ringed` when it is the result; named by its English.
pub(crate) fn outcome_word(
    ui: &mut egui::Ui,
    outcome: Outcome,
    (tappable, ringed): (bool, bool),
) -> egui::Response {
    let words = Words::from(t::outcome(outcome));
    let shown = words.shown(ui.ctx());
    let font = TextStyle::Button.resolve(ui.style());
    let ink = match (ringed, tappable) {
        (true, _) => INK_ACTIVE,
        (false, true) => INK,
        (false, false) => INK_SECONDARY,
    };
    let galley = ui.painter().layout_no_wrap(shown.to_owned(), font, ink);
    let width = galley.size().x + 2.0 * MARK + 5.0 * SPACE;
    let sense = if tappable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, tappable, ringed, shown));
    controls::register(ui, words.id(), rect);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let outline = Outline::pill(rect);
    let painter = ui.painter();
    if tappable && response.is_pointer_button_down_on() {
        paint_pressed(painter, outline);
    }
    if ringed {
        paint_mark(painter, outline, 1.0);
    }
    let left = outline.rect.left() + SPACE + MARK;
    paint_result(
        painter,
        (egui::pos2(left, rect.center().y), MARK),
        outcome,
        ink,
    );
    let text = Rect::from_min_size(
        egui::pos2(left + MARK + SPACE, rect.center().y - galley.size().y / 2.0),
        galley.size(),
    );
    painter.galley(text.min, galley, ink);
    response
}
