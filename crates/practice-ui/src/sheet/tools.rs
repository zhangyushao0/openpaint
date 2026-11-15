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
    BrushProjection, EditorAction, HistoryIntent, Intent, SessionProjection, Tool, ToolIntent,
};
use openpaint_core::Color;
use openpaint_practice::{Fill, ToolSet};
use openpaint_practice_app::{Pen, PracticeIntent, PracticeProjection};
use openpaint_ui::components::{
    Source, UNDER_ISLANDS, card, choices, icon_button, icon_toggle, swatch, text_button,
    text_choice,
};
use openpaint_ui::design::{Icon, SPACE, TOUCH_TARGET};
use openpaint_ui::taps::push_once;

use super::SheetTaps;
use super::marks::{level_colour, value_colour};
use crate::text as t;

/// All tools, in the left island after Gallery; none on a long colour copy, which shows the full
/// interface only (S104). Its taps ask the painting (`painting`) and the sheet.
pub(crate) fn all_tools(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    (painting, sheet): (&mut Vec<Intent>, &mut Vec<PracticeIntent>),
) {
    if practice.kind.is_some_and(|kind| kind.facts().tools.full) {
        return;
    }
    choices(ui, "all tools", |ui, group| {
        let on = practice.all_tools;
        if text_choice(ui, group, t::ALL_TOOLS, on).clicked() {
            push_once(sheet, PracticeIntent::AllTools(!on));
            if on {
                // Back to the exercise's tools: the pens draw again, the toolbox's brush tool on.
                painting.push(Intent::Tool(ToolIntent::Use(Tool::Brush)));
            }
        }
    });
}

/// The shown exercise's own tools beside its frame, under the band at the screen's right, laid
/// out from its kind's facts (`ToolSet`): pens of its own (a line's, a division's, value masses'
/// greys) with Fill and Undo and Redo under them; a whole copy's layered tools; or the wheel with
/// Fill and the painting pens. Returns where they lie, in points.
pub(crate) fn exercise_tools(
    ctx: &egui::Context,
    practice: &PracticeProjection,
    taps: &mut SheetTaps<'_, '_>,
) -> Option<egui::Rect> {
    let projection = taps.lent.session;
    let shown = (projection, practice);
    if practice.current.is_none() || practice.all_tools {
        return None;
    }
    let source = (Source::Within(Align::Center), UNDER_ISLANDS);
    let facts = practice.kind?.facts();
    let tools = &facts.tools;
    if tools.full {
        return None;
    }
    if tools.layered {
        let laid = |ui: &mut egui::Ui| layered_tools(ui, tools, shown, taps);
        return Some(card(ctx, "practice pens", source, laid).1);
    }
    if !tools.pens.is_empty() {
        let id = if tools.greys() {
            "practice greys"
        } else {
            "practice pens"
        };
        let pens = |ui: &mut egui::Ui| pens_tools(ui, tools, shown, taps);
        return Some(card(ctx, id, source, pens).1);
    }
    let brush = projection.brush()?;
    // A painted copy compared, or any How with its demonstration: the card needs the room; the
    // wheel comes back with the pen or Got it. A swatch's wheel stays while compared.
    let painted = !facts.picks.swatch;
    let wheel = tools.wheel && !(practice.comparing && painted) && practice.how.is_none();
    let ((), rect) = card(ctx, "practice colour", source, |ui| {
        if wheel {
            ui.vertical_centered(|ui| taps.lent.inline_wheel(ui, brush));
            ui.add_space(SPACE);
        }
        colour_rows(ui, (tools, brush), shown, taps);
    });
    Some(rect)
}

/// The canvas's projection and its practice sheet's.
type Shown<'a> = (&'a SessionProjection, &'a PracticeProjection);

/// The words of `fill`.
const fn fill_words(fill: Fill) -> t::Text {
    match fill {
        Fill::Frame => t::FILL_FRAME,
        Fill::Cell => t::FILL_CELL,
    }
}

/// Tools of its own pens' (S95, S96): the pens, then Fill (in the grey chosen where its pens are
/// greys) with Undo and Redo.
fn pens_tools(
    ui: &mut egui::Ui,
    tools: &ToolSet,
    (projection, practice): Shown<'_>,
    taps: &mut SheetTaps<'_, '_>,
) {
    ui.horizontal(|ui| pens(ui, ("pens", tools.pens), practice, taps));
    let colour = if tools.greys() {
        (practice.pen)
            .and_then(|(pen, _)| pen.value())
            .and_then(|value| openpaint_practice_app::value_colour(value).ok())
    } else {
        projection.brush().map(|brush| brush.color)
    };
    ui.horizontal(|ui| {
        if let (Some(fill), Some(colour)) = (tools.fill, colour) {
            fill_frame(ui, colour, fill_words(fill), taps.sheet);
        }
        if tools.history {
            history(ui, projection, taps.lent.taps);
        }
    });
}

/// A colour's, a combination's or a value scale's rows under the wheel: Fill frame or Fill cell
/// with Undo and Redo, then the painting pens and the eyedropper (your own copy may be picked
/// from, V大; the original only after Compare). Where it does not paint (a value scale's cells
/// are filled, never painted, S100) the eyedropper lies beside Redo.
fn colour_rows(
    ui: &mut egui::Ui,
    (tools, brush): (&ToolSet, &BrushProjection),
    (projection, practice): Shown<'_>,
    taps: &mut SheetTaps<'_, '_>,
) {
    ui.horizontal(|ui| {
        if let Some(fill) = tools.fill {
            fill_frame(ui, brush.color, fill_words(fill), taps.sheet);
        }
        if tools.history {
            history(ui, projection, taps.lent.taps);
        }
        if tools.eyedropper && !tools.paints {
            eyedropper(ui, brush, taps.lent.taps);
        }
    });
    if tools.paints {
        ui.horizontal(|ui| {
            pens(ui, ("pens", &Pen::PAINTING), practice, taps);
            if tools.eyedropper {
                eyedropper(ui, brush, taps.lent.taps);
            }
        });
    }
}

/// A whole copy's tools (S95, S96): its pens on their layers, then the painting pens for its
/// colour blocks with the eyedropper, Undo and Redo; while a painting pen is chosen the wheel
/// over them (unless the comparison or How needs the room) and Fill frame beside Undo.
fn layered_tools(
    ui: &mut egui::Ui,
    tools: &ToolSet,
    (projection, practice): Shown<'_>,
    taps: &mut SheetTaps<'_, '_>,
) {
    let painting = practice.pen.is_some_and(|(pen, _)| pen.paints());
    let brush = projection.brush().filter(|_| painting);
    let room = !practice.comparing && practice.how.is_none();
    if let Some(brush) = brush.filter(|_| tools.wheel && room) {
        ui.vertical_centered(|ui| taps.lent.inline_wheel(ui, brush));
        ui.add_space(SPACE);
    }
    ui.horizontal(|ui| pens(ui, ("pens", tools.pens), practice, taps));
    if tools.paints {
        ui.horizontal(|ui| {
            pens(ui, ("block pens", &Pen::PAINTING), practice, taps);
            if let Some(brush) = projection.brush().filter(|_| tools.eyedropper) {
                eyedropper(ui, brush, taps.lent.taps);
            }
        });
    }
    ui.horizontal(|ui| {
        if let (Some(fill), Some(brush)) = (tools.fill, brush) {
            fill_frame(ui, brush.color, fill_words(fill), taps.sheet);
        }
        if tools.history {
            history(ui, projection, taps.lent.taps);
        }
    });
}

/// The eyedropper, on or off.
fn eyedropper(ui: &mut egui::Ui, brush: &BrushProjection, taps: &mut Vec<Intent>) {
    let picking = brush.picking;
    if icon_toggle(ui, Icon::Eyedropper, t::EYEDROPPER, picking).clicked() {
        push_once(taps, Intent::Tool(ToolIntent::Eyedropper(!picking)));
    }
}

/// The colour the fill takes as a swatch beside `words`, a tap on either filling.
fn fill_frame(ui: &mut egui::Ui, colour: Color, words: t::Text, taps: &mut Vec<PracticeIntent>) {
    ui.add_space(SPACE);
    let picked = swatch(ui, colour, t::COLOUR_PICKED, TOUCH_TARGET, false);
    let tapped = text_button(ui, words, true);
    if picked.clicked() || tapped.clicked() {
        push_once(taps, PracticeIntent::Fill(colour));
    }
}

/// The exercise's pens of `offered` (the blue one only once compared), a group of choices named
/// `id` (a whole copy's two rows two groups, each its own mark).
fn pens(
    ui: &mut egui::Ui,
    (id, offered): (&str, &[Pen]),
    practice: &PracticeProjection,
    taps: &mut SheetTaps<'_, '_>,
) {
    let chosen = practice.pen.map(|(pen, _)| pen);
    let compared = practice.result.is_some();
    // Abstract analysis's levels mark a picture: 强, 次, 弱 (S112).
    let marks = (practice.kind).is_some_and(|kind| kind.facts().tools.marks);
    choices(ui, id, |ui, group| {
        for &pen in offered {
            if pen == Pen::Correct && !compared {
                continue;
            }
            let word = text_choice(ui, group, t::pen_of(pen, marks), chosen == Some(pen));
            // A division's level shows its colour under its word (S95), a grey its grey (S96).
            let colour = (pen.level())
                .map(level_colour)
                .or_else(|| pen.value().map(value_colour));
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
                push_once(taps.sheet, PracticeIntent::Pen(pen));
                // The pens are the brush tool's: a chosen eraser gives way.
                taps.lent
                    .taps
                    .push(Intent::Tool(ToolIntent::Use(Tool::Brush)));
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
