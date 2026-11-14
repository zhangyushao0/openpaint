//! A key shift's demonstration (`demo.rs`, S107): an example crop of five areas shifted toward an
//! orange by rule (`key_shift_rule.rs`), painted in V大's order, a step each: the tendency looked
//! at (ringed); the largest colour first, Fill frame, the whole picture shifted; the grey, which
//! moves most and gains a little chroma; the blue across from the orange, moved a little and
//! greyer; the light cream, barely moved; the ochre last, its edge with its nearest neighbour
//! pencilled, still apart. Under the frames a chip shows the area being painted, its colour, an
//! arrow and its shifted colour; under the copy the tendency's disc. Compare: both frames in grey,
//! the lightness kept.

use crate::colour_areas::areas_generated::{Arrangement, GeneratedAreas};
use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, Strip, clearest};
use crate::kinds::key_shifts::key_shift::{Lean, Tendency};
use crate::maths::colour::{Rgb8, delta_e, from_oklab, in_gamut, oklab};

const KEY_SEED: u64 = 0xDE30_5EED_0107;
/// Seconds the tendency is looked at before anything is painted.
const LOOK: f64 = 0.9;
/// The example's tendency: a pure orange.
const TENDENCY: Tendency = Tendency {
    lean: Lean::WarmPure,
    hue: 60,
};
/// The example's colours by its areas' sizes, largest first (Oklab lightness, chroma, hue): a
/// muted red-brown base near the tendency, a cool grey, a blue across from it, an ochre, a cream.
const PALETTE: [[f64; 3]; 5] = [
    [0.55, 0.07, 20.0],
    [0.62, 0.012, 240.0],
    [0.47, 0.1, 250.0],
    [0.74, 0.09, 95.0],
    [0.93, 0.025, 95.0],
];
/// The palette's places in the order painted.
const PAINTED: [usize; 5] = [0, 1, 2, 4, 3];

/// Under its frames (`Strip`): each area's chip from when its painting begins, its colour and its
/// shifted colour, and the tendency's disc under the copy, ringed while it is looked at.
pub(crate) fn key_shift() -> Demo {
    let (example, painted) = example();
    let areas = &example.areas;
    let shifted = TENDENCY.shifted(areas);
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, flat(areas.colours.clone()));
    script.step();
    let looked = script.at;
    script.at += LOOK;
    let mut chips = Vec::new();
    let mut shown = vec![shifted[painted[0]]; areas.count()];
    for (step, area) in painted.iter().enumerate() {
        if step > 0 {
            shown[*area] = shifted[*area];
        }
        script.step();
        chips.push((script.at, [areas.colours[*area], shifted[*area]]));
        script.draw(On::Copy, Marker::Ink, flat(shown.clone()));
    }
    // The last painted against its nearest neighbour: still apart.
    let last = painted[painted.len() - 1];
    let nearest = (areas.neighbours.iter())
        .filter(|(a, b)| *a == last || *b == last)
        .min_by(|x, y| {
            let apart = |(a, b): &&(usize, usize)| delta_e(shifted[*a], shifted[*b]);
            apart(x).total_cmp(&apart(y))
        });
    if let Some((a, b)) = nearest {
        let edges = areas.edges(*a, Some(*b)).into_iter();
        script.together(
            edges
                .map(|e| (On::Copy, Marker::Pencil, Drawn::Line(e)))
                .collect(),
        );
    }
    let grey = |colours: &[Rgb8]| -> Vec<Rgb8> {
        (colours.iter())
            .map(|c| from_oklab([oklab(*c)[0], 0.0, 0.0]))
            .collect()
    };
    let compared = vec![
        (On::Original, Marker::Given, flat(grey(&areas.colours))),
        (On::Copy, Marker::Given, flat(grey(&shifted))),
    ];
    let mut demo = script.compare(DemoLayout::Pair, compared);
    demo.strip = Some(Box::new(Strip {
        disc: TENDENCY.colour(),
        looked,
        chips,
    }));
    demo.areas = Some(example);
    demo
}

/// The example: of a fixed seed's crops of blocks, one of five areas whose largest holds about
/// two fifths, its areas coloured by `PALETTE` from the largest down, every edge hard; and its
/// areas in the order painted.
fn example() -> (GeneratedAreas, Vec<usize>) {
    let mut example = clearest(
        (KEY_SEED, 12),
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
    let painted = (PAINTED.iter()).filter_map(|role| order.get(*role).copied());
    (example, painted.collect())
}

/// The example's areas in `colours`, every edge hard.
const fn flat(colours: Vec<Rgb8>) -> Drawn {
    Drawn::Areas {
        colours,
        soft: false,
    }
}
