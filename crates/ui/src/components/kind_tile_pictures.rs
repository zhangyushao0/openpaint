//! The tiles' pictures of the kinds that copy a whole picture (S95, `kind_tile.rs`): a division (a
//! picture's edge, a long line right through it, a bent one meeting its sides and a part's small
//! frame standing on it) and a whole copy (the same edge, a ridge, a field's edge, a tree and a
//! house), in `TILE`'s units.

use openpaint_practice::Family;

use super::{Mark, closed, curve, square};

/// A picture's edge in the tile.
const EDGE: [[f32; 2]; 2] = [[9.0, 8.0], [45.0, 32.0]];

/// `family`'s picture as marks; none for another kind.
pub(super) fn picture(family: Family) -> Vec<Mark> {
    let mut marks = vec![Mark::Line(closed(&square(EDGE[0], EDGE[1])))];
    match family {
        Family::Division => marks.extend([
            Mark::Line(vec![[9.0, 18.0], [45.0, 14.5]]),
            Mark::Line(vec![[9.0, 26.0], [27.0, 23.0], [45.0, 26.5]]),
            Mark::Line(vec![[33.0, 24.4], [33.0, 17.0], [40.0, 17.0], [40.0, 25.3]]),
        ]),
        Family::WholeCopy => marks.extend([
            Mark::Line(vec![[9.0, 19.0], [22.0, 12.5], [45.0, 18.0]]),
            Mark::Line(vec![[9.0, 26.5], [45.0, 25.5]]),
            Mark::Line(closed(&square([31.0, 20.5], [38.0, 25.8]))),
            Mark::Line(vec![[29.8, 20.5], [34.5, 16.5], [39.2, 20.5]]),
            Mark::Line(curve(|t| {
                let turn = std::f32::consts::TAU * t;
                [16.0 + 3.6 * turn.cos(), 19.5 + 4.2 * turn.sin()]
            })),
            Mark::Line(vec![[16.0, 23.7], [16.0, 26.3]]),
        ]),
        _ => return Vec::new(),
    }
    marks
}
