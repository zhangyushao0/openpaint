//! A practice sheet's own tools (S76): only what the exercise uses, placed by it, beside the empty
//! frame at the screen's right (the room the view leaves), with the full interface one tap away
//! and back. The top band keeps only Gallery, All tools and the practice bar, so nothing meets
//! in portrait.
//!
//! - All tools: a word after Gallery in the left island; while the full interface shows it
//!   carries the ink pill and the usual islands and side bar are back (the toolbox's brush
//!   draws).
//! - A line: the pens, Sketch (a light pencil for marks and construction lines) and Draw, and
//!   after Compare Correct (blue); under them Undo and Redo.
//! - A colour: the colour wheel (the colour panel's own) and Fill frame, which fills the frame
//!   with the colour picked in one tap.

use egui::Align;
use openpaint_app::{
    EditorAction, HistoryIntent, Intent, Pen, PracticeIntent, PracticeProjection,
    SessionProjection, Tool, ToolIntent,
};

use crate::color_panel::ColorPanel;
use crate::components::{
    Source, UNDER_ISLANDS, card, choices, icon_button, swatch, text_button, text_choice,
};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::taps::push_once;

/// The pens' words, in the island's order.
const PENS: [(Pen, &str); 3] = [
    (Pen::Sketch, "Sketch"),
    (Pen::Draw, "Draw"),
    (Pen::Correct, "Correct"),
];

/// Whether the exercise's own tools show on `projection`'s sheet (not the full interface).
pub(crate) fn own_tools(projection: &SessionProjection) -> bool {
    projection
        .practice()
        .is_some_and(|practice| !practice.all_tools)
}

/// All tools, in the left island after Gallery.
pub(crate) fn all_tools(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<Intent>) {
    choices(ui, "all tools", |ui, group| {
        let on = practice.all_tools;
        if text_choice(ui, group, "All tools", on).clicked() {
            push_once(taps, Intent::Practice(PracticeIntent::AllTools(!on)));
            if on {
                // Back to the exercise's tools: the pens draw again, the toolbox's brush tool on.
                taps.push(Intent::Tool(ToolIntent::Use(Tool::Brush)));
            }
        }
    });
}

/// The shown exercise's own tools beside its frame, under the band at the screen's right: a
/// line's pens, undo and redo, or a colour's wheel and Fill frame. Returns where they lie, in
/// points.
pub(crate) fn exercise_tools(
    ctx: &egui::Context,
    projection: &SessionProjection,
    colors: &mut ColorPanel,
    taps: &mut Vec<Intent>,
) -> Option<egui::Rect> {
    let practice = projection.practice()?;
    if practice.current.is_none() || practice.all_tools {
        return None;
    }
    let source = (Source::Within(Align::Center), UNDER_ISLANDS);
    if practice.line {
        let pens = |ui: &mut egui::Ui| line_tools(ui, projection, taps);
        return Some(card(ctx, "practice pens", source, pens).1);
    }
    let brush = projection.brush()?;
    let ((), rect) = card(ctx, "practice colour", source, |ui| {
        ui.vertical_centered(|ui| colors.inline_wheel(ui, brush, taps));
        ui.add_space(SPACE);
        ui.horizontal(|ui| {
            ui.add_space(SPACE);
            let fill = Intent::Practice(PracticeIntent::Fill(brush.color));
            let picked = swatch(ui, brush.color, "Colour picked", TOUCH_TARGET, false);
            let tapped = text_button(ui, "Fill frame", true);
            if picked.clicked() || tapped.clicked() {
                push_once(taps, fill);
            }
        });
    });
    Some(rect)
}

/// A line's pens, and under them Undo and Redo.
fn line_tools(ui: &mut egui::Ui, projection: &SessionProjection, taps: &mut Vec<Intent>) {
    let Some(practice) = projection.practice() else {
        return;
    };
    let chosen = practice.pen.map(|(pen, _)| pen);
    let compared = practice.result.is_some();
    ui.horizontal(|ui| {
        choices(ui, "pens", |ui, group| {
            for (pen, word) in PENS {
                if pen == Pen::Correct && !compared {
                    continue;
                }
                if text_choice(ui, group, word, chosen == Some(pen)).clicked() {
                    push_once(taps, Intent::Practice(PracticeIntent::Pen(pen)));
                    // The pens are the brush tool's: a chosen eraser gives way.
                    taps.push(Intent::Tool(ToolIntent::Use(Tool::Brush)));
                }
            }
        });
    });
    ui.horizontal(|ui| {
        let history = [
            (Icon::Undo, "Undo", EditorAction::Undo, HistoryIntent::Undo),
            (Icon::Redo, "Redo", EditorAction::Redo, HistoryIntent::Redo),
        ];
        for (icon, label, action, intent) in history {
            let enabled = projection.availability(action).is_offered();
            if icon_button(ui, icon, label, enabled).clicked() {
                taps.push(Intent::History(intent));
            }
        }
    });
}
