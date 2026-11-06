//! A practice sheet's own tools (S76): only what the exercise uses, placed by it, beside the empty
//! frame at the screen's right (the room the view leaves), with the full interface one tap away
//! and back. The top band keeps only Gallery, All tools and the practice bar, so nothing meets
//! in portrait.
//!
//! - All tools: a word after Gallery in the left island; while the full interface shows it
//!   carries the ink pill and the usual islands and side bar are back (the toolbox's brush
//!   draws). A long colour copy has no tools of its own and no All tools: it is painted in the
//!   full interface with the toolbox's own brushes, opening on Hard Paint (S104).
//! - A line, combined lines or a single shape: the pens, Sketch (a light pencil for marks and
//!   construction lines) and Draw, and after Compare Correct (blue); under them Undo and Redo. A
//!   division: its levels' pens, Main, Second, Third (S95); a whole copy those and Draw.
//! - A colour: the colour wheel (the colour panel's own) and Fill frame, which fills the frame
//!   with the colour picked in one tap.
//! - A colour combination: the wheel, Fill frame (the overall tone first) with Undo and Redo,
//!   the painting pens Big, Small and Soft in the colour picked and the eyedropper (the copy may
//!   be picked from); a value scale the wheel, Fill cell (the cell in hand, which the pen touching
//!   a cell takes), Undo, Redo and the eyedropper, no pens (S100). While a painted copy is
//!   compared the wheel gives its room to the comparison's card, and while How shows to its card
//!   (S92).

use egui::Align;
use openpaint_app::{
    BrushProjection, EditorAction, HistoryIntent, Intent, Pen, PracticeIntent, PracticeProjection,
    SessionProjection, Tool, ToolIntent,
};
use openpaint_core::Color;
use openpaint_practice::Family;

use crate::color_panel::ColorPanel;
use crate::components::{
    Source, UNDER_ISLANDS, card, choices, icon_button, icon_toggle, swatch, text_button,
    text_choice,
};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::taps::push_once;
use crate::text as t;

/// Whether the exercise's own tools show on `projection`'s sheet (not the full interface).
pub(crate) fn own_tools(projection: &SessionProjection) -> bool {
    projection
        .practice()
        .is_some_and(|practice| !practice.all_tools)
}

/// All tools, in the left island after Gallery; none on a long colour copy, which shows the full
/// interface only (S104).
pub(crate) fn all_tools(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<Intent>) {
    if practice.family == Some(Family::Study) {
        return;
    }
    choices(ui, "all tools", |ui, group| {
        let on = practice.all_tools;
        if text_choice(ui, group, t::ALL_TOOLS, on).clicked() {
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
    let family = practice.family?;
    if family == Family::WholeCopy {
        let tools = |ui: &mut egui::Ui| whole_copy_tools(ui, projection, colors, taps);
        return Some(card(ctx, "practice pens", source, tools).1);
    }
    if family.drawn() {
        let offered: &[Pen] = match family {
            Family::Division => &Pen::DIVIDING,
            _ => &Pen::DRAWING,
        };
        let pens = |ui: &mut egui::Ui| {
            ui.horizontal(|ui| pens(ui, ("pens", offered), projection, taps));
            ui.horizontal(|ui| history(ui, projection, taps));
        };
        return Some(card(ctx, "practice pens", source, pens).1);
    }
    if family == Family::ValueMasses {
        let greys = |ui: &mut egui::Ui| masses_tools(ui, projection, taps);
        return Some(card(ctx, "practice greys", source, greys).1);
    }
    let brush = projection.brush()?;
    // A painted copy compared, or any How with its demonstration: the card needs the room; the
    // wheel comes back with the pen or Got it.
    let wheel = !(practice.comparing && family.painted()) && practice.instructions.is_none();
    let ((), rect) = card(ctx, "practice colour", source, |ui| {
        if wheel {
            ui.vertical_centered(|ui| colors.inline_wheel(ui, brush, taps));
            ui.add_space(SPACE);
        }
        colour_rows(ui, (family, brush), projection, taps);
    });
    Some(rect)
}

/// A colour's, a combination's or a value scale's rows under the wheel: Fill frame or Fill cell
/// with Undo and Redo, then the painting pens and the eyedropper (your own copy may be picked
/// from, V大; the original only after Compare). A value scale's cells are filled, never painted
/// (S100): no pens, the eyedropper beside Redo.
fn colour_rows(
    ui: &mut egui::Ui,
    (family, brush): (Family, &BrushProjection),
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    let fill = match family {
        Family::Scale => Some(t::FILL_CELL),
        Family::Colour | Family::Areas | Family::KeyShift => Some(t::FILL_FRAME),
        _ => None,
    };
    let pick = |ui: &mut egui::Ui, taps: &mut Vec<Intent>| {
        let picking = brush.picking;
        if icon_toggle(ui, Icon::Eyedropper, t::EYEDROPPER, picking).clicked() {
            push_once(taps, Intent::Tool(ToolIntent::Eyedropper(!picking)));
        }
    };
    ui.horizontal(|ui| {
        if let Some(words) = fill {
            fill_frame(ui, brush.color, words, taps);
        }
        if family.painted() {
            history(ui, projection, taps);
        }
        if family == Family::Scale {
            pick(ui, taps);
        }
    });
    if family.painted() && family != Family::Scale {
        ui.horizontal(|ui| {
            pens(ui, ("pens", &Pen::PAINTING), projection, taps);
            pick(ui, taps);
        });
    }
}

/// A whole copy's tools (S95, S96): its division's pens and Draw, then the painting pens for its
/// colour blocks with the eyedropper, Undo and Redo; while a painting pen is chosen the wheel
/// over them (unless the comparison or How needs the room) and Fill frame beside Undo.
fn whole_copy_tools(
    ui: &mut egui::Ui,
    projection: &SessionProjection,
    colors: &mut ColorPanel,
    taps: &mut Vec<Intent>,
) {
    let Some(practice) = projection.practice() else {
        return;
    };
    let painting = practice.pen.is_some_and(|(pen, _)| pen.paints());
    let brush = projection.brush().filter(|_| painting);
    let room = !practice.comparing && practice.instructions.is_none();
    if let Some(brush) = brush.filter(|_| room) {
        ui.vertical_centered(|ui| colors.inline_wheel(ui, brush, taps));
        ui.add_space(SPACE);
    }
    ui.horizontal(|ui| pens(ui, ("pens", &Pen::COPYING), projection, taps));
    ui.horizontal(|ui| {
        pens(ui, ("block pens", &Pen::PAINTING), projection, taps);
        if let Some(brush) = projection.brush() {
            let picking = brush.picking;
            if icon_toggle(ui, Icon::Eyedropper, t::EYEDROPPER, picking).clicked() {
                push_once(taps, Intent::Tool(ToolIntent::Eyedropper(!picking)));
            }
        }
    });
    ui.horizontal(|ui| {
        if let Some(brush) = brush {
            fill_frame(ui, brush.color, t::FILL_FRAME, taps);
        }
        history(ui, projection, taps);
    });
}

/// Value masses' tools (S96): the three greys, Fill frame in the grey chosen, Undo and Redo.
fn masses_tools(ui: &mut egui::Ui, projection: &SessionProjection, taps: &mut Vec<Intent>) {
    let Some(practice) = projection.practice() else {
        return;
    };
    ui.horizontal(|ui| pens(ui, ("pens", &Pen::VALUES), projection, taps));
    let grey = (practice.pen)
        .and_then(|(pen, _)| pen.value())
        .and_then(|value| openpaint_app::value_colour(value).ok());
    ui.horizontal(|ui| {
        if let Some(grey) = grey {
            fill_frame(ui, grey, t::FILL_FRAME, taps);
        }
        history(ui, projection, taps);
    });
}

/// The colour the fill takes as a swatch beside `words`, a tap on either filling.
fn fill_frame(ui: &mut egui::Ui, colour: Color, words: t::Text, taps: &mut Vec<Intent>) {
    ui.add_space(SPACE);
    let picked = swatch(ui, colour, t::COLOUR_PICKED, TOUCH_TARGET, false);
    let tapped = text_button(ui, words, true);
    if picked.clicked() || tapped.clicked() {
        push_once(taps, Intent::Practice(PracticeIntent::Fill(colour)));
    }
}

/// The exercise's pens of `offered` (the blue one only once compared), a group of choices named
/// `id` (a whole copy's two rows two groups, each its own mark).
fn pens(
    ui: &mut egui::Ui,
    (id, offered): (&str, &[Pen]),
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    let Some(practice) = projection.practice() else {
        return;
    };
    let chosen = practice.pen.map(|(pen, _)| pen);
    let compared = practice.result.is_some();
    choices(ui, id, |ui, group| {
        for &pen in offered {
            if pen == Pen::Correct && !compared {
                continue;
            }
            let word = text_choice(ui, group, t::pen(pen), chosen == Some(pen));
            // A division's level shows its colour under its word (S95), a grey its grey (S96).
            let colour = (pen.level())
                .map(crate::practice_division_marks::level_colour)
                .or_else(|| pen.value().map(crate::practice_marks::value_colour));
            if let Some(colour) = colour {
                let rect = word.rect;
                let under = rect.center().y + 1.4 * SPACE;
                let ends = [rect.left() + 1.5 * SPACE, rect.right() - 1.5 * SPACE];
                let line = ends.map(|x| egui::pos2(x, under));
                let width = if pen.value().is_some() { 4.0 } else { 2.0 };
                ui.painter()
                    .line_segment(line, egui::Stroke::new(width, colour));
            }
            if word.clicked() {
                push_once(taps, Intent::Practice(PracticeIntent::Pen(pen)));
                // The pens are the brush tool's: a chosen eraser gives way.
                taps.push(Intent::Tool(ToolIntent::Use(Tool::Brush)));
            }
        }
    });
}

/// Undo and Redo.
fn history(ui: &mut egui::Ui, projection: &SessionProjection, taps: &mut Vec<Intent>) {
    let history = [
        (Icon::Undo, t::UNDO, EditorAction::Undo, HistoryIntent::Undo),
        (Icon::Redo, t::REDO, EditorAction::Redo, HistoryIntent::Redo),
    ];
    for (icon, label, action, intent) in history {
        let enabled = projection.availability(action).is_offered();
        if icon_button(ui, icon, label, enabled).clicked() {
            taps.push(Intent::History(intent));
        }
    }
}
