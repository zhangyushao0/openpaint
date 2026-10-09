//! The brush picker's list (S47, Procreate's brush library cut to five fixed sets): under the
//! heading the sets as words in two columns (five do not fit one row of the panel at the body's
//! 18 points), the one shown in the ink pill, the mark sliding when another is tapped; the
//! picker opens on the set holding the chosen brush. Under them that set's brushes with their
//! pictures, the chosen one outlined (in any set it lies in; none while its set is not shown);
//! Recent with nothing drawn yet says so in the quiet hint. In the four sets the hand orders, a
//! row held still lifts and follows the hand, the others making room, and lands where it is let
//! go (`row_drag.rs`, as layer rows); Recent is never ordered. A tap chooses a brush; the chosen
//! brush's row tapped again opens its editor. StreamLine under the list.

use std::collections::HashMap;

use openpaint_app::{BrushIntent, BrushKey, BrushProjection, BrushRow, BrushSet, Intent, Tool};

use crate::components::{
    BRUSH_ROW, BrushRowView, brush_row, choices, hint, slider, text_choice_wide,
};
use crate::design::{SPACE, TOUCH_TARGET};
use crate::row_drag::{self, RowDrag, held};

/// A brush row carried by the hand: its brush, landing at a place of its set (0 first).
pub(crate) type BrushDrag = RowDrag<BrushKey, usize>;

/// What Recent says before anything was drawn.
pub(crate) const RECENT_EMPTY: &str = "Brushes you draw with appear here.";

/// The list's own state, kept by the panel.
#[derive(Debug, Default)]
pub(crate) struct ListState {
    /// The set shown; `None`: the set holding the chosen brush, as the picker opens.
    pub(crate) shown: Option<BrushSet>,
    pub(crate) drag: Option<BrushDrag>,
}

impl ListState {
    /// The set the list shows for `brush`.
    pub(crate) fn set(&self, brush: &BrushProjection) -> BrushSet {
        self.shown.unwrap_or(brush.set)
    }
}

/// The list: the heading, the sets' words, the shown set's rows, StreamLine. Returns whether the
/// chosen brush's row was tapped again, which opens its editor.
pub(crate) fn list(
    ui: &mut egui::Ui,
    brush: &BrushProjection,
    (state, textures): (&mut ListState, &HashMap<BrushKey, egui::TextureHandle>),
    taps: &mut Vec<Intent>,
) -> bool {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.heading(match brush.tool {
            Tool::Brush => "Brushes",
            Tool::Smudge => "Smudge",
            Tool::Eraser => "Eraser",
        });
    });
    if let Some(set) = set_words(ui, state.set(brush)) {
        state.shown = Some(set);
        state.drag = None;
    }
    let set = state.set(brush);
    let rows = brush.rows(set);
    let edit = if rows.is_empty() {
        if set == BrushSet::Recent {
            hint(ui, RECENT_EMPTY);
        }
        false
    } else {
        brush_rows(ui, (brush, set, rows), (&mut state.drag, textures), taps)
    };
    streamline(ui, brush.streamline.amount(), taps);
    edit
}

/// The five sets as words in two columns, `shown` in the ink pill. Returns the set tapped, if
/// another was.
pub(crate) fn set_words(ui: &mut egui::Ui, shown: BrushSet) -> Option<BrushSet> {
    let half = ((ui.available_width() - 2.0 * SPACE) / 2.0).floor();
    let mut tapped = None;
    choices(ui, "brush sets", |ui, group| {
        for pair in BrushSet::ALL.chunks(2) {
            ui.horizontal(|ui| {
                ui.add_space(2.0 * SPACE);
                for &set in pair {
                    let on = set == shown;
                    let word = text_choice_wide(ui, group, set.label(), (true, on), half);
                    if word.clicked() && !on {
                        tapped = Some(set);
                    }
                }
            });
        }
    });
    tapped
}

/// The rows of `set`, the carried one where it would land. Returns whether the chosen brush's
/// row was tapped again.
fn brush_rows(
    ui: &mut egui::Ui,
    (brush, set, rows): (&BrushProjection, BrushSet, &[BrushRow]),
    (drag, textures): (
        &mut Option<BrushDrag>,
        &HashMap<BrushKey, egui::TextureHandle>,
    ),
    taps: &mut Vec<Intent>,
) -> bool {
    let top = ui.cursor().top();
    let (hand, down) = row_drag::hand(ui, top);
    // A drag of a row this set no longer holds (the list changed under it) ends.
    let dropped = carry(drag, rows, (hand, down), taps);
    // The row let go lies where it landed from this pass on, before the toolbox lists it so.
    let order = shown(rows, drag.as_ref().or(dropped.as_ref()));
    let ordered = set != BrushSet::Recent;
    let mut edit = false;
    for (index, row) in order.iter().enumerate() {
        let carried = drag.as_ref().filter(|d| d.key == row.key);
        let lifted = carried
            .zip(hand)
            .map(|(d, hand)| d.top(hand).max(0.0) - index as f32 * BRUSH_ROW);
        let selected = row.key == brush.current;
        let view = BrushRowView {
            key: egui::Id::new(("brush row", set.label(), row.key.as_str())),
            name: &row.name,
            preview: textures.get(&row.key).map(egui::TextureHandle::id),
            selected,
            lifted,
        };
        let response = brush_row(ui, &view);
        if drag.is_none() && dropped.is_none() && ordered && held(ui, &response) {
            let grab = hand.map_or(0.0, |hand| hand - (response.rect.top() - top));
            let places = (0..rows.len()).collect();
            *drag = RowDrag::lift(row.key.clone(), grab, places, index);
        }
        // A carried row's lift is no tap.
        if drag.is_some() || dropped.is_some() || !response.clicked() {
            continue;
        }
        if selected {
            edit = true;
        } else {
            push(taps, BrushIntent::Choose(row.key.clone()));
        }
    }
    edit
}

/// The carried row follows the hand; let go (`down` false), it lands where it shows. Returns the
/// drag let go this pass.
fn carry(
    drag: &mut Option<BrushDrag>,
    rows: &[BrushRow],
    (hand, down): (Option<f32>, bool),
    taps: &mut Vec<Intent>,
) -> Option<BrushDrag> {
    if drag
        .as_ref()
        .is_some_and(|d| !rows.iter().any(|row| row.key == d.key))
    {
        *drag = None;
    }
    if let (Some(carried), Some(hand), true) = (drag.as_mut(), hand, down) {
        carried.follow((hand, BRUSH_ROW), |place| *place);
    }
    if down {
        return None;
    }
    let dropped = drag.take()?;
    if dropped.moved() {
        let key = dropped.key.clone();
        push(
            taps,
            BrushIntent::Reorder {
                key,
                to: dropped.place,
            },
        );
    }
    Some(dropped)
}

/// `rows` with the carried one at the place it would land.
fn shown<'a>(rows: &'a [BrushRow], drag: Option<&BrushDrag>) -> Vec<&'a BrushRow> {
    let mut order: Vec<&BrushRow> = rows.iter().collect();
    if let Some(drag) = drag
        && let Some(from) = order.iter().position(|row| row.key == drag.key)
    {
        let carried = order.remove(from);
        order.insert(drag.place.min(order.len()), carried);
    }
    order
}

/// The chosen brush's StreamLine under the rows (Procreate keeps it in the brush's settings):
/// its name and value, then a slider from off to the strongest. It applies from the next
/// stroke on and is the brush's own, per tool.
fn streamline(ui: &mut egui::Ui, amount: f32, taps: &mut Vec<Intent>) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.label("StreamLine");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(2.0 * SPACE);
            ui.label(format!("{} %", (amount * 100.0).round()));
        });
    });
    if let Some(change) = slider(ui, "StreamLine", amount, true) {
        push(
            taps,
            BrushIntent::StreamLine {
                amount: change.value,
                settled: change.settled,
            },
        );
    }
}

/// egui may run a pass twice; one tap or value is sent once.
fn push(taps: &mut Vec<Intent>, intent: BrushIntent) {
    let intent = Intent::Brush(intent);
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}

#[cfg(test)]
#[path = "brush_list_tests.rs"]
mod tests;
