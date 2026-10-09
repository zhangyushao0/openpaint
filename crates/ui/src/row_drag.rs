//! Carrying a row of a list (S38's layer rows, S47's brush rows; Procreate's press and hold): a
//! contact that stays still on a row for `HOLD` lifts it; the row then follows the hand and the
//! list lays its subject out where it would land, the other rows making room (settle); letting
//! go puts it there. What a row carries (`K`) and where it may land (`P`) are the list's: the
//! layers' places come from the app (into and out of groups), a brush set's are its rows. The
//! list does not scroll while a row is carried.

/// How long a contact rests on a row before the row lifts, in seconds.
const HOLD: f64 = 0.4;

/// A row carried by the hand.
#[derive(Debug, Clone)]
pub(crate) struct RowDrag<K, P> {
    /// What the row shows.
    pub(crate) key: K,
    /// The hand's height in the row when it lifted, in points.
    grab: f32,
    /// Where it may land, in the list's order; `start` is where it lies.
    places: Vec<P>,
    start: usize,
    /// Where it lands if let go now.
    pub(crate) place: P,
}

impl<K, P: Copy + PartialEq> RowDrag<K, P> {
    /// The row of `key` lifts with the hand `grab` points below its top; it may land at
    /// `places`, of which `start` is where it lies. `None` when `start` is no place.
    pub(crate) fn lift(key: K, grab: f32, places: Vec<P>, start: usize) -> Option<Self> {
        let place = *places.get(start)?;
        Some(Self {
            key,
            grab,
            places,
            start,
            place,
        })
    }

    /// Follows the hand at `hand` points below the list's top: the place whose row (`row_of`:
    /// rows from the list's top with the subject there) lies nearest the carried row. Two places
    /// that share a row (a layer into and out of a group at its bottom edge) lie half a row
    /// apart, the farther one half a row on.
    pub(crate) fn follow(&mut self, (hand, pitch): (f32, f32), row_of: impl Fn(&P) -> usize) {
        let top = hand - self.grab;
        let start = self.places[self.start];
        let mut best = (f32::INFINITY, start);
        let mut consider = |place: P, ideal: f32| {
            let distance = (ideal - top).abs();
            if distance < best.0 {
                best = (distance, place);
            }
        };
        let here = row_of(&start) as f32 * pitch;
        consider(start, here);
        for (way, sign) in [
            (&self.places[self.start + 1..], -1.0),
            (&self.places[..self.start], 1.0),
        ] {
            let ordered: Vec<&P> = if sign < 0.0 {
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

    /// It would land somewhere else than where it lies.
    pub(crate) fn moved(&self) -> bool {
        self.place != self.places[self.start]
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

/// The hand: how far below `top` (points) it is, if anywhere, and whether it presses.
pub(crate) fn hand(ui: &egui::Ui, top: f32) -> (Option<f32>, bool) {
    ui.input(|input| {
        let at = input.pointer.interact_pos().or(input.pointer.latest_pos());
        (at.map(|at| at.y - top), input.pointer.any_down())
    })
}
