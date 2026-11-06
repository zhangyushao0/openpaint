//! The tiles' pictures of the kinds that copy a whole picture (S95, `kind_tile.rs`): a division (a
//! picture's edge, a long line right through it, a bent one meeting its sides and a part's small
//! frame standing on it), a whole copy (the same edge, a ridge, a field's edge, a tree and a
//! house) and value masses (S96: the same edge, its ground under the ridge tinted middle, a
//! crown and a roof dark, the sky and a wall left light), and a key shift (S107: a crop's bands,
//! an arrow, the tendency's disc), in `TILE`'s units.

use openpaint_practice::Family;

use super::{Mark, closed, curve, square};

/// Value masses' tints: the middle and the dark, shares of the tile's colour.
const MIDDLE: f32 = 0.3;
const DARK: f32 = 0.75;

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
        Family::ValueMasses => marks.extend(masses()),
        _ => return Vec::new(),
    }
    marks
}

/// A key shift (S107): a crop's two bands, an arrow, the tendency's disc it shifts toward.
pub(super) fn shift() -> Vec<Mark> {
    let crop = square([7.0, 10.0], [27.0, 30.0]);
    let disc: Vec<[f32; 2]> = (0..12)
        .map(|k| {
            let turn = std::f32::consts::TAU * k as f32 / 12.0;
            [42.0 + 5.0 * turn.cos(), 20.0 + 5.0 * turn.sin()]
        })
        .collect();
    vec![
        Mark::Tint(
            vec![[7.0, 10.0], [27.0, 10.0], [27.0, 17.0], [7.0, 21.0]],
            0.15,
        ),
        Mark::Tint(
            vec![[7.0, 21.0], [27.0, 17.0], [27.0, 30.0], [7.0, 30.0]],
            0.45,
        ),
        Mark::Line(closed(&crop)),
        Mark::Line(vec![[7.0, 21.0], [27.0, 17.0]]),
        Mark::Line(vec![[30.0, 20.0], [35.0, 20.0]]),
        Mark::Line(vec![[33.0, 18.0], [35.0, 20.0], [33.0, 22.0]]),
        Mark::Tint(disc.clone(), 0.6),
        Mark::Line(closed(&disc)),
    ]
}

/// Value masses' blocks: the ground middle, a crown and a roof dark, a wall light on the ground.
fn masses() -> Vec<Mark> {
    let ground = vec![
        [9.0, 19.0],
        [22.0, 12.5],
        [45.0, 18.0],
        [45.0, 32.0],
        [9.0, 32.0],
    ];
    let crown: Vec<[f32; 2]> = (0..8)
        .map(|k| {
            let turn = std::f32::consts::TAU * k as f32 / 8.0;
            [16.0 + 4.0 * turn.cos(), 20.0 + 4.6 * turn.sin()]
        })
        .collect();
    let roof = vec![[29.8, 21.0], [34.5, 16.5], [39.2, 21.0]];
    let wall = square([31.0, 21.0], [38.0, 26.5]);
    vec![
        Mark::Tint(ground.clone(), MIDDLE),
        Mark::Tint(crown.clone(), DARK),
        Mark::Tint(roof.clone(), DARK),
        // The wall lies light over the ground: the paper, its outline drawn.
        Mark::Paper(wall.to_vec()),
        Mark::Line(vec![ground[0], ground[1], ground[2]]),
        Mark::Line(closed(&crown)),
        Mark::Line(closed(&roof)),
        Mark::Line(closed(&wall)),
    ]
}
