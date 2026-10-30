//! The solids' tiles' pictures (S93, `kind_tile.rs`): an upright cylinder (its top whole, its
//! bottom's front, two sides), an upright cone (its base's front, two sides from its point) and a
//! sphere with its middle section's front, in `TILE`'s units.

use openpaint_practice::Family;

use super::{Mark, curve};

/// The arc of the ellipse about `centre` with `axes` from `from` to `to` degrees (y down).
fn arc(centre: [f32; 2], axes: [f32; 2], (from, to): (f32, f32)) -> Mark {
    Mark::Line(curve(|t| {
        let turn = (from + (to - from) * t).to_radians();
        [
            centre[0] + axes[0] * turn.cos(),
            centre[1] + axes[1] * turn.sin(),
        ]
    }))
}

/// `family`'s solid as marks; none for another kind.
pub(super) fn solid(family: Family) -> Vec<Mark> {
    match family {
        Family::Cylinder => vec![
            arc([27.0, 11.0], [11.0, 4.0], (0.0, 360.0)),
            arc([27.0, 30.0], [11.0, 4.0], (0.0, 180.0)),
            Mark::Line(vec![[16.0, 11.0], [16.0, 30.0]]),
            Mark::Line(vec![[38.0, 11.0], [38.0, 30.0]]),
        ],
        // The sides touch the base where lines from the point do: 10.4 degrees above its middle.
        Family::Cone => vec![
            arc([27.0, 30.0], [12.0, 4.5], (-10.4, 190.4)),
            Mark::Line(vec![[27.0, 5.0], [38.8, 29.2]]),
            Mark::Line(vec![[27.0, 5.0], [15.2, 29.2]]),
        ],
        Family::Sphere => vec![
            arc([27.0, 20.0], [13.0, 13.0], (0.0, 360.0)),
            arc([27.0, 21.5], [13.0, 4.5], (0.0, 180.0)),
        ],
        _ => Vec::new(),
    }
}
