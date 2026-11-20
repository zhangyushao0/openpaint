//! The replay's icons: Play, a triangle pointing right, its weight a little right of the middle so
//! it reads centred; Pause, two upright bars.

use super::icons::{Icon, Line};

pub(super) fn media_icons(icon: Icon) -> Vec<Line> {
    match icon {
        Icon::Play => vec![vec![[0.36, 0.24], [0.76, 0.5], [0.36, 0.76], [0.36, 0.24]]],
        Icon::Pause => vec![
            vec![[0.38, 0.26], [0.38, 0.74]],
            vec![[0.62, 0.26], [0.62, 0.74]],
        ],
        _ => Vec::new(),
    }
}
