//! A gamut mask's demonstration (S153): an example crop of five areas painted again inside
//! Gurney's triangle toward an orange, in the order the walk-through paints it, a step each: the
//! mask looked at (ringed); the frame filled with the mask's middle at the largest area's
//! lightness; the big areas by their lightness, each moved inside the mask (the blue across from
//! the orange greyed); the small ones last, still apart; Compare: both frames in grey, the
//! lightness kept. Under the frames a chip shows the area being painted, its colour, an arrow and
//! its colour inside the mask; under the copy the mask on its disc.

use super::mask::{Mask, MaskShape, nearest};
use crate::colour_areas::areas_generated::{Arrangement, GeneratedAreas};
use crate::demos::demo::{Aimed, Demo, DemoLayout, Drawn, Marker, On, Script, Strip, clearest};
use crate::maths::colour::{Rgb8, from_oklab, in_gamut, oklab};

const MASK_SEED: u64 = 0xDE30_5EED_0153;
/// Seconds the mask is looked at before anything is painted.
const LOOK: f64 = 0.9;
/// The example's mask: Gurney's triangle, its apex an orange.
const MASK: Mask = Mask {
    shape: MaskShape::Triangle,
    hue: 60,
};
/// How far into the mask a colour outside it is moved past its edge: a share of the way from its
/// nearest point to the mask's middle, so it lies plainly inside.
const INTO: f64 = 0.25;
/// The example's colours by its areas' sizes, largest first (Oklab lightness, chroma, hue): a
/// red-brown, a cool grey, a blue across from the orange, a green, a cream.
const PALETTE: [[f64; 3]; 5] = [
    [0.56, 0.08, 25.0],
    [0.66, 0.015, 240.0],
    [0.45, 0.1, 250.0],
    [0.6, 0.1, 145.0],
    [0.92, 0.03, 95.0],
];
/// How many of the largest areas the big areas' step paints.
const BIG: usize = 2;

/// Under its frames (`Strip`): each area's chip from when its painting begins, its colour and its
/// colour inside the mask, and the mask's disc under the copy, ringed while it is looked at.
pub(crate) fn gamut_mask() -> Demo {
    let (example, order) = example();
    let areas = &example.areas;
    let masked: Vec<Rgb8> = areas.colours.iter().map(|c| inside(*c)).collect();
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, flat(areas.colours.clone()));
    script.step();
    let looked = script.at;
    script.at += LOOK;
    // The frame filled with the mask's middle at the largest area's lightness.
    script.step();
    let [ma, mb] = MASK.middle();
    let tone = from_oklab([oklab(areas.colours[order[0]])[0], ma, mb]);
    let mut shown = vec![tone; areas.count()];
    script.draw(On::Copy, Marker::Ink, flat(shown.clone()));
    let mut chips = Vec::new();
    for (painted, area) in order.iter().enumerate() {
        if painted == 0 || painted == BIG {
            script.step();
        }
        shown[*area] = masked[*area];
        chips.push((script.at, [areas.colours[*area], masked[*area]]));
        script.draw(On::Copy, Marker::Ink, flat(shown.clone()));
    }
    let grey = |colours: &[Rgb8]| -> Vec<Rgb8> {
        (colours.iter())
            .map(|c| from_oklab([oklab(*c)[0], 0.0, 0.0]))
            .collect()
    };
    let compared = vec![
        (On::Original, Marker::Given, flat(grey(&areas.colours))),
        (On::Copy, Marker::Given, flat(grey(&masked))),
    ];
    let mut demo = script.compare(DemoLayout::Pair, compared);
    demo.strip = Some(Box::new(Strip {
        aimed: Aimed::Mask(MASK.corners()),
        looked,
        chips,
    }));
    demo.areas = Some(example);
    demo
}

/// `colour` inside the example's mask, its lightness kept: as it is where it lies inside, else
/// moved from its nearest point of the mask a share `INTO` toward the mask's middle.
pub(crate) fn inside(colour: Rgb8) -> Rgb8 {
    let [l, a, b] = oklab(colour);
    let corners = MASK.corners();
    let q = nearest(&corners, [a, b]);
    if q == [a, b] {
        return colour;
    }
    let m = MASK.middle();
    let [x, y] = [q[0] + INTO * (m[0] - q[0]), q[1] + INTO * (m[1] - q[1])];
    in_gamut([l, x.hypot(y), y.atan2(x).to_degrees()])
}

/// The example: of a fixed seed's crops of blocks, one of five areas whose largest holds about
/// two fifths, its areas coloured by `PALETTE` from the largest down, every edge hard; and its
/// areas from the largest down, the order painted.
fn example() -> (GeneratedAreas, Vec<usize>) {
    let mut example = clearest(
        (MASK_SEED, 12),
        |seed| GeneratedAreas::generate(Arrangement::Blocks, seed),
        |generated| {
            let sizes = generated.areas.sizes();
            let largest = sizes.iter().max().copied().unwrap_or(0) as f64;
            let share = largest / sizes.iter().sum::<usize>().max(1) as f64;
            let five = if generated.areas.count() == 5 {
                0.0
            } else {
                1.0
            };
            -five - (share - 0.4).abs()
        },
    );
    let mut order: Vec<usize> = (0..example.areas.count()).collect();
    let sizes = example.areas.sizes();
    order.sort_by(|a, b| sizes[*b].cmp(&sizes[*a]));
    for (role, area) in order.iter().enumerate() {
        example.areas.colours[*area] = in_gamut(PALETTE[role.min(PALETTE.len() - 1)]);
    }
    example.areas.soft.fill(0.0);
    (example, order)
}

/// The example's areas in `colours`, every edge hard.
const fn flat(colours: Vec<Rgb8>) -> Drawn {
    Drawn::Areas {
        colours,
        soft: false,
    }
}
