//! The colour kinds' demonstrations (`demo.rs`): a colour (the wheel's ring to the hue's family,
//! then lighter or darker, then purer or greyer, the frame filled), a colour combination (the
//! overall tone, the largest area first, each neighbour against it, the weak contrast kept weak)
//! and a value scale of as many cells as the scale in hand (S97: their order numbered under them,
//! the middle cell first, then each half's middle, then the rest, the pick moving only up and down
//! the wheel's square), each then compared.

use super::{Demo, Drawn, Layout, Marker, On, Script, clearest};
use crate::areas::{Areas, Arrangement, GRID};
use crate::colour::{Rgb8, delta_e, from_hsv, from_oklab, hsv, oklab};
use crate::colour_match::{Lightness, Purity, Tone};
use crate::path::Point;
use crate::plan::Family;
use crate::scale::{ROW, ScaleTone, ValueScale, halving};

const COLOUR_SEED: u64 = 0xDE30_5EED_0004;
const AREAS_SEED: u64 = 0xDE30_5EED_0005;
const SCALE_SEED: u64 = 0xDE30_5EED_0006;
/// Where a value scale's order is numbered under its cells, under the even row Compare lays.
const NUMBERED: f64 = 0.87;
/// The part of the scale's frame its example shows: the row, the even row and the numbers.
const SCALE_VIEW: [Point; 2] = [[0.02, 0.27], [0.98, 0.93]];
/// How far off the hand's copy of a cell is in lightness, in turn along the row: a little (under a
/// quarter step of nine cells); and the cell it paints last this share of a step, over a quarter,
/// so Compare outlines it with its even one.
const HAND: [f64; 4] = [-0.008, 0.006, 0.01, -0.006];
const HAND_OFF: f64 = 0.35;

/// The rectangle from `[left, top]` to `[right, bottom]` as two rows.
fn rect([left, top]: Point, [right, bottom]: Point) -> Vec<[Point; 2]> {
    vec![
        [[left, top], [right, top]],
        [[left, bottom], [right, bottom]],
    ]
}

fn band(rows: Vec<[Point; 2]>, colour: Rgb8) -> Drawn {
    Drawn::Band {
        rows,
        colour: Some(colour),
    }
}

/// A single colour.
pub(super) fn colour() -> Demo {
    let tone = Tone {
        purity: Purity::Muted,
        lightness: Lightness::Middle,
    };
    let original = tone.colour(COLOUR_SEED);
    let [h, s, v] = hsv(original);
    let mut script = Script::new();
    let whole = rect([0.0, 0.0], [1.0, 1.0]);
    script.given(On::Original, Marker::Given, band(whole.clone(), original));
    script.given(
        On::Wheel,
        Marker::Given,
        Drawn::Pick([(h + 140.0) % 360.0, 0.85, 0.9]),
    );
    // V大's order: the hue first, then lightness, then purity. A first pick by eye a little off,
    // as a hand makes it: its hue a step round, a little darker and purer.
    let hue = (h + 12.0) % 360.0;
    let picks = [
        [hue, 0.85, 0.9],
        [hue, 0.85, v - 0.06],
        [hue, s + 0.07, v - 0.06],
    ];
    for pick in picks {
        script.step();
        script.draw(On::Wheel, Marker::Ink, Drawn::Pick(pick));
    }
    let picked = from_hsv(picks[2]);
    script.step();
    script.draw(On::Copy, Marker::Ink, band(whole, picked));
    let patch = rect([0.25, 0.25], [0.75, 0.75]);
    let compared = vec![
        (On::Copy, Marker::Given, band(patch.clone(), original)),
        (On::Original, Marker::Given, band(patch, picked)),
        (On::Wheel, Marker::Answer, Drawn::Ringed([h, s, v])),
    ];
    script.compare(Family::Colour, Layout::Wheel, compared)
}

/// A colour combination.
pub(super) fn areas() -> Demo {
    // Four or five bands of middle lightness, so each area and the weak contrast read small.
    let areas = clearest(
        (AREAS_SEED, 8),
        |seed| Areas::generate(Arrangement::Strata, seed),
        |areas| {
            let count = areas.count() as f64;
            let light = areas.colours.iter().map(|c| oklab(*c)[0]).sum::<f64>() / count;
            -(light - 0.66).abs() - if count > 5.0 { 0.2 } else { 0.0 }
        },
    );
    let count = areas.count();
    let sizes = sizes(&areas);
    let (order, weak, last) = painting(&areas, &sizes);
    // A hand's copy: each area a little lighter or darker.
    let copied: Vec<Rgb8> = (0..count)
        .map(|a| {
            let [l, x, y] = oklab(areas.colours[a]);
            let off = if a % 2 == 0 { 0.012 } else { -0.01 };
            from_oklab([l + off, x, y])
        })
        .collect();
    let tone = overall(&areas, &sizes);
    let mut script = Script::new();
    script.given(
        On::Original,
        Marker::Given,
        Drawn::Areas(areas.colours.clone()),
    );
    let mut shown = vec![tone; count];
    script.step();
    script.draw(On::Copy, Marker::Ink, Drawn::Areas(shown.clone()));
    for (turn, area) in order.iter().enumerate() {
        if turn < 2 {
            script.step();
        }
        shown[*area] = copied[*area];
        script.draw(On::Copy, Marker::Ink, Drawn::Areas(shown.clone()));
    }
    script.step();
    if let (Some((a, b)), Some(last)) = (weak, last) {
        for edge in areas.edges(a, Some(b)) {
            script.draw(On::Copy, Marker::Pencil, Drawn::Line(edge));
        }
        shown[last] = areas.colours[last];
        script.draw(On::Copy, Marker::Ink, Drawn::Areas(shown));
    }
    let mut compared = vec![(On::Copy, Marker::Faint, Drawn::Areas(areas.colours.clone()))];
    for on in [On::Original, On::Copy] {
        for edge in areas.edges(order[0], None) {
            compared.push((on, Marker::Given, Drawn::Line(edge)));
        }
    }
    let mut demo = script.compare(Family::Areas, Layout::Pair, compared);
    demo.areas = Some(areas);
    demo
}

/// The overall tone: the areas' colours by how much of the frame each holds.
fn overall(areas: &Areas, sizes: &[usize]) -> Rgb8 {
    let total: usize = sizes.iter().sum::<usize>().max(1);
    std::array::from_fn(|k| {
        let sum: usize = (0..areas.count())
            .map(|a| usize::from(areas.colours[a][k]) * sizes[a])
            .sum();
        u8::try_from(sum / total).unwrap_or(u8::MAX)
    })
}

/// The areas in the order painted, the largest first (the weak pair's second left out); the
/// weak pair; and that second, painted last against its partner.
fn painting(areas: &Areas, sizes: &[usize]) -> (Vec<usize>, Option<(usize, usize)>, Option<usize>) {
    let weak = (areas.neighbours.iter())
        .min_by(|x, y| {
            let apart = |(a, b): &&(usize, usize)| delta_e(areas.colours[*a], areas.colours[*b]);
            apart(x).total_cmp(&apart(y))
        })
        .copied();
    let mut order: Vec<usize> = (0..areas.count()).collect();
    order.sort_by(|a, b| sizes[*b].cmp(&sizes[*a]));
    let last = weak.map(|(a, b)| {
        let at = |x: usize| order.iter().position(|o| *o == x).unwrap_or(0);
        if at(a) > at(b) { a } else { b }
    });
    let largest = order[0];
    order.retain(|a| Some(*a) != last || *a == largest);
    (order, weak, last)
}

/// How many of the map's cells each area holds.
fn sizes(areas: &Areas) -> Vec<usize> {
    let mut sizes = vec![0; areas.count()];
    for row in 0..GRID {
        for column in 0..GRID {
            if let Some(size) = sizes.get_mut(areas.cell([column, row])) {
                *size += 1;
            }
        }
    }
    sizes
}

/// A value scale of `cells` cells (the scale in hand's count; never its colours), painted in
/// V大's order of halving.
pub(super) fn scale(cells: usize) -> Demo {
    let scale = ValueScale::generate(ScaleTone::Hue, cells, SCALE_SEED);
    let rounds = halving(scale.cells);
    let mut script = Script::new();
    scale_given(&mut script, &scale, &rounds);
    // The copy's lightness per cell: the ends given, the others a little off as an eye judges,
    // the last painted over a quarter step.
    let targets = scale.targets();
    let step = (targets[scale.cells - 1] - targets[0]) / (scale.cells - 1) as f64;
    let last = rounds.last().and_then(|round| round.last()).copied();
    let values: Vec<f64> = (targets.iter().enumerate())
        .map(|(cell, target)| match cell {
            _ if cell == 0 || cell + 1 == scale.cells => *target,
            _ if Some(cell) == last => target + HAND_OFF * step,
            _ => target + HAND[cell % HAND.len()],
        })
        .collect();
    // A round at a time: the pick moved up or down the wheel's square to its first cell, then
    // the round's cells painted, each its own step of lightness.
    for round in &rounds {
        script.step();
        let colour = |cell: usize| scale.colour(values[cell]);
        let pick = Drawn::Pick(hsv(colour(round[0])));
        script.draw(On::Wheel, Marker::Ink, pick);
        script.together(
            (round.iter())
                .map(|cell| {
                    let [a, b] = scale.cell(*cell);
                    (On::Copy, Marker::Ink, band(rect(a, b), colour(*cell)))
                })
                .collect(),
        );
    }
    let compared = scale_compared(&scale, &values);
    let mut demo = script.compare(Family::Scale, Layout::Row, compared);
    demo.view = Some(SCALE_VIEW);
    demo
}

/// What a value scale's exercise gives: its cells with the ends filled, the order to paint them
/// numbered under them in pencil (`rounds`, each cell its round's number), the wheel picking the
/// dark end.
fn scale_given(script: &mut Script, scale: &ValueScale, rounds: &[Vec<usize>]) {
    let [dark, light] = scale.ends();
    for cell in 0..scale.cells {
        let [a, b] = scale.cell(cell);
        let outline = vec![a, [b[0], a[1]], b, [a[0], b[1]], a];
        script.given(On::Copy, Marker::Rule, Drawn::Line(outline));
    }
    for (end, colour) in [(0, dark), (scale.cells - 1, light)] {
        let [a, b] = scale.cell(end);
        script.given(On::Copy, Marker::Given, band(rect(a, b), colour));
    }
    for (place, round) in rounds.iter().enumerate() {
        for cell in round {
            let [[left, _], [right, _]] = scale.cell(*cell);
            let at = [(left + right) / 2.0, NUMBERED];
            let number = u8::try_from(place + 1).unwrap_or(u8::MAX);
            script.given(On::Copy, Marker::Pencil, Drawn::Numeral(at, number));
        }
    }
    script.given(On::Wheel, Marker::Given, Drawn::Pick(hsv(dark)));
}

/// A value scale compared: the even scale as a row under the copy's, cell under cell, and a cell
/// of the copy's (`values`) a quarter step off or more outlined red with its even one.
fn scale_compared(scale: &ValueScale, values: &[f64]) -> Vec<(On, Marker, Drawn)> {
    let check = scale.check(values.iter().map(|value| Some(*value)).collect());
    let mut compared = Vec::new();
    for (cell, target) in check.targets.iter().enumerate() {
        let [a, b] = scale.even_cell(cell);
        let colour = scale.colour(*target);
        compared.push((On::Copy, Marker::Given, band(rect(a, b), colour)));
    }
    let [_, top, _, _] = ROW;
    for cell in &check.uneven {
        let [[left, _], [right, bottom]] = scale.even_cell(*cell);
        let outline = vec![
            [left, top],
            [right, top],
            [right, bottom],
            [left, bottom],
            [left, top],
        ];
        compared.push((On::Copy, Marker::Answer, Drawn::Line(outline)));
    }
    compared
}
