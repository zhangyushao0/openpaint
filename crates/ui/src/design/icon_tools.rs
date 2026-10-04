//! The tools' icons: what the pen does, each held at a slant with its tip down to the left (the
//! smudge's fingertip too), the Selection tool's lasso and the Transform tool's pointer.

use std::f32::consts::PI;

use super::icons::{ARC_STEPS, Icon, Line, arc, curve};

/// The tools held at a slant, tip down to the left.
pub(super) fn tools(icon: Icon) -> Vec<Line> {
    match icon {
        Icon::Brush => {
            // The handle, the ferrule across it, then the bristles swelling and closing to a
            // point.
            let (a, b, tip) = ([0.46, 0.4], [0.6, 0.54], [0.14, 0.86]);
            let mut bristles = curve(b, [0.56, 0.78], tip);
            bristles.extend(curve(tip, [0.22, 0.44], a).into_iter().skip(1));
            vec![vec![[0.86, 0.14], [0.6, 0.4]], vec![a, b], bristles]
        }
        Icon::Smudge => smudge(),
        Icon::Eraser => vec![
            vec![
                [0.2, 0.58],
                [0.54, 0.24],
                [0.82, 0.52],
                [0.48, 0.86],
                [0.2, 0.58],
            ],
            vec![[0.33, 0.45], [0.61, 0.73]],
            vec![[0.6, 0.86], [0.84, 0.86]],
        ],
        Icon::Eyedropper => vec![
            // The tube from its tip, the collar, then the bulb.
            vec![
                [0.18, 0.82],
                [0.21, 0.7],
                [0.5, 0.41],
                [0.59, 0.5],
                [0.3, 0.79],
                [0.18, 0.82],
            ],
            vec![[0.42, 0.36], [0.64, 0.58]],
            {
                let mut bulb = vec![[0.49, 0.43]];
                bulb.extend(arc([0.67, 0.33], 0.06, 1.25 * PI, 2.25 * PI, ARC_STEPS));
                bulb.push([0.57, 0.51]);
                bulb
            },
        ],
        Icon::Selection => lasso(),
        // A pointer: the tip, down its left side, a notch, the tail and back up its right side.
        Icon::Transform => vec![vec![
            [0.3, 0.16],
            [0.3, 0.78],
            [0.45, 0.64],
            [0.56, 0.86],
            [0.66, 0.81],
            [0.55, 0.6],
            [0.75, 0.6],
            [0.3, 0.16],
        ]],
        _ => Vec::new(),
    }
}

/// A finger from the upper right, its round tip down to the left: up one side, round the tip,
/// back the other side; a crease across it; under the tip, the wave of colour it smeared.
fn smudge() -> Vec<Line> {
    let (base, tip, half) = ([0.8, 0.2], [0.38, 0.62], 0.11);
    let side = half * std::f32::consts::FRAC_1_SQRT_2;
    let along = |p: [f32; 2], k: f32| [p[0] + side * k, p[1] + side * k];
    let mut outline = vec![along(base, 1.0), along(tip, 1.0)];
    outline.extend(
        arc(tip, half, 0.25 * PI, 1.25 * PI, ARC_STEPS)
            .into_iter()
            .skip(1),
    );
    outline.push(along(base, -1.0));
    let crease = [
        base[0] + 0.3 * (tip[0] - base[0]),
        base[1] + 0.3 * (tip[1] - base[1]),
    ];
    vec![
        outline,
        vec![along(crease, 0.6), along(crease, -0.6)],
        curve([0.14, 0.82], [0.3, 0.9], [0.5, 0.84]),
    ]
}

/// A loop drawn in dashes, the way a selection's path is drawn before it closes, with the
/// lasso's tail curling down to the left.
fn lasso() -> Vec<Line> {
    let (centre, radii) = ([0.54, 0.42], [0.32, 0.22]);
    let dashes = 9;
    let mut lines: Vec<Line> = (0..dashes)
        .map(|dash| {
            let start = 2.0 * PI * dash as f32 / dashes as f32;
            let end = start + 2.0 * PI * 0.6 / dashes as f32;
            (0..=3)
                .map(|step| {
                    let angle = start + (end - start) * step as f32 / 3.0;
                    [
                        centre[0] + radii[0] * angle.cos(),
                        centre[1] + radii[1] * angle.sin(),
                    ]
                })
                .collect()
        })
        .collect();
    let knot = [centre[0] - 0.22, centre[1] + 0.16];
    lines.push(curve(knot, [0.16, 0.72], [0.3, 0.86]));
    lines
}
