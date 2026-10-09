//! Dragging a layer row (S38, Procreate's press and hold, `row_drag.rs`): a row held still lifts;
//! it then follows the hand and the list shows the layer where it would land, the other rows
//! making room (settle); letting go moves it there as one step. The places it can land are the
//! app's (`LayerList::places`: the steps move up and down take, into and out of groups); the
//! panel only lays them out and picks the one nearest the hand.

use std::collections::HashSet;

use openpaint_app::{Intent, LayerIntent, LayerList, LayerRow};
use openpaint_core::{GroupId, LayerId, LayerPlace};

use crate::row_drag::RowDrag;

/// A layer row carried by the hand.
pub(crate) type Drag = RowDrag<LayerId, LayerPlace>;

/// The row of `layer` lifts with the hand `grab` points below its top.
pub(crate) fn lift(list: &LayerList, layer: LayerId, grab: f32) -> Option<Drag> {
    let places = list.places(layer);
    let start = places.iter().position(|place| place.by == 0)?;
    RowDrag::lift(layer, grab, places, start)
}

/// Follows the hand at `hand` points below the list's top (`row_drag::RowDrag::follow`), the
/// layer's row counted with group rows and masks, the closed groups in `collapsed`.
pub(crate) fn follow(
    drag: &mut Drag,
    list: &LayerList,
    (hand, pitch): (f32, f32),
    collapsed: &HashSet<GroupId>,
) {
    let layer = drag.key;
    drag.follow((hand, pitch), |place| {
        slot(&placed(list, layer, *place), layer, collapsed)
    });
}

/// Let go: the move, if it lands anywhere new.
pub(crate) fn drop(drag: &Drag) -> Option<Intent> {
    (drag.place.by != 0).then_some(Intent::Layer(LayerIntent::Move {
        layer: drag.key,
        by: drag.place.by,
    }))
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
