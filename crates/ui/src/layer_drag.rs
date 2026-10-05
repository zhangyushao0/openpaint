//! Dragging a layer row (S38, Procreate's press and hold): a contact that stays still on a row
//! for `HOLD` lifts it; the row then follows the hand and the list shows the layer where it
//! would land, the other rows making room (settle); letting go moves it there as one step. The
//! places it can land are the app's (`LayerList::places`: the steps move up and down take,
//! into and out of groups); the panel only lays them out and picks the one nearest the hand.

use std::collections::HashSet;

use openpaint_app::{Intent, LayerIntent, LayerList, LayerRow};
use openpaint_core::{GroupId, LayerId, LayerPlace};

/// How long a contact rests on a row before the row lifts, in seconds.
const HOLD: f64 = 0.4;

/// A row carried by the hand.
#[derive(Debug, Clone)]
pub(crate) struct Drag {
    pub(crate) layer: LayerId,
    /// The hand's height in the row when it lifted, in points.
    grab: f32,
    places: Vec<LayerPlace>,
    /// Where it lands if let go now.
    pub(crate) place: LayerPlace,
}

impl Drag {
    /// The row of `layer` lifts with the hand `grab` points below its top.
    pub(crate) fn lift(list: &LayerList, layer: LayerId, grab: f32) -> Option<Self> {
        let places = list.places(layer);
        let place = *places.iter().find(|place| place.by == 0)?;
        Some(Self {
            layer,
            grab,
            places,
            place,
        })
    }

    /// Follows the hand at `hand` points below the list's top: the place whose row lies nearest
    /// the carried row. Two places that share a row (into and out of a group at its bottom
    /// edge) lie half a row apart, the farther one half a row on.
    pub(crate) fn follow(
        &mut self,
        list: &LayerList,
        (hand, pitch): (f32, f32),
        collapsed: &HashSet<GroupId>,
    ) {
        let top = hand - self.grab;
        let row_of =
            |place: &LayerPlace| slot(&placed(list, self.layer, *place), self.layer, collapsed);
        let start = self.places.iter().position(|p| p.by == 0).unwrap_or(0);
        let mut best = (f32::INFINITY, self.places[start]);
        let mut consider = |place: LayerPlace, ideal: f32| {
            let distance = (ideal - top).abs();
            if distance < best.0 {
                best = (distance, place);
            }
        };
        let here = row_of(&self.places[start]) as f32 * pitch;
        consider(self.places[start], here);
        for (way, sign) in [
            (&self.places[start + 1..], -1.0),
            (&self.places[..start], 1.0),
        ] {
            let ordered: Vec<&LayerPlace> = if sign < 0.0 {
                way.iter().collect()
            } else {
                way.iter().rev().collect()
            };
            let (mut last_row, mut last_ideal) = (here, here);
            for place in ordered {
                let row = row_of(place) as f32 * pitch;
                let ideal = if row == last_row {
                    last_ideal + sign * pitch / 2.0
                } else {
                    row
                };
                consider(*place, ideal);
                (last_row, last_ideal) = (row, ideal);
            }
        }
        self.place = best.1;
    }

    /// Where the carried row shows: `hand` less the grab, below the list's top.
    pub(crate) fn top(&self, hand: f32) -> f32 {
        hand - self.grab
    }

    /// Let go: the move, if it lands anywhere new.
    pub(crate) fn drop(&self) -> Option<Intent> {
        (self.place.by != 0).then_some(Intent::Layer(LayerIntent::Move {
            layer: self.layer,
            by: self.place.by,
        }))
    }
}

/// Whether the contact pressing `row` has rested on it long enough to lift it, without moving
/// as far as a scroll. While it rests, frames keep coming so the moment is seen.
pub(crate) fn held(ui: &egui::Ui, row: &egui::Response) -> bool {
    if !row.is_pointer_button_down_on() {
        return false;
    }
    let (rested, still) = ui.input(|input| {
        let pointer = &input.pointer;
        let since = pointer
            .press_start_time()
            .map_or(0.0, |start| input.time - start);
        (since, pointer.could_any_button_be_click())
    });
    if still && rested < HOLD {
        ui.ctx().request_repaint();
    }
    still && rested >= HOLD
}

/// `list` as it shows with `layer` at `place`: the others keep their order and groups.
pub(crate) fn placed(list: &LayerList, layer: LayerId, place: LayerPlace) -> LayerList {
    let mut rows: Vec<LayerRow> = list.rows.to_vec();
    let Some(from) = rows.iter().position(|row| row.id == layer) else {
        return list.clone();
    };
    let mut moved = rows.remove(from);
    moved.group = place.group;
    rows.insert(place.index.min(rows.len()), moved);
    LayerList {
        rows: rows.into(),
        ..list.clone()
    }
}

/// The row (from the list's top, group rows and masks counted) where `layer` shows in `list`.
/// The carried layer shows even inside a closed group.
pub(crate) fn slot(list: &LayerList, layer: LayerId, collapsed: &HashSet<GroupId>) -> usize {
    let mut slot = 0;
    let mut above = None;
    for row in list.rows.iter().rev() {
        if row.group != above && row.group.and_then(|id| list.group(id)).is_some() {
            slot += 1;
        }
        above = row.group;
        if row.id == layer {
            return slot;
        }
        if row.group.is_some_and(|id| collapsed.contains(&id)) {
            continue;
        }
        slot += 1 + usize::from(row.mask.is_some());
    }
    slot
}
