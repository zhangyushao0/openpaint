//! The layer panel's own icons: a folder for groups, the chevron that opens and closes one,
//! and the bent arrow of a Clipping Mask, pointing down to the base it clips to.

use std::f32::consts::{FRAC_PI_2, PI};

use super::icons::{Icon, Line, arc};

pub(super) fn layer_icons(icon: Icon) -> Vec<Line> {
    match icon {
        Icon::Group => folder(),
        Icon::Expanded => vec![vec![[0.3, 0.4], [0.5, 0.6], [0.7, 0.4]]],
        Icon::Collapsed => vec![vec![[0.4, 0.3], [0.6, 0.5], [0.4, 0.7]]],
        Icon::Clipped => {
            // From the right along the top, round the corner and down to the head.
            let mut shaft = vec![[0.76, 0.3]];
            shaft.extend(arc([0.46, 0.42], 0.12, -FRAC_PI_2, -PI, 6));
            shaft.push([0.34, 0.74]);
            vec![shaft, vec![[0.22, 0.6], [0.34, 0.74], [0.46, 0.6]]]
        }
        _ => Vec::new(),
    }
}

/// A folder: its tab on the upper left, the body a rounded box.
fn folder() -> Vec<Line> {
    vec![vec![
        [0.18, 0.74],
        [0.18, 0.28],
        [0.42, 0.28],
        [0.5, 0.36],
        [0.82, 0.36],
        [0.82, 0.74],
        [0.18, 0.74],
    ]]
}
