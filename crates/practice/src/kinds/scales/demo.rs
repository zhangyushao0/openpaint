//! A value scale's demonstration (`demos/demo.rs`) of as many cells as the scale in hand (S97):
//! their order numbered under them, the middle cell first, then each half's middle, then the rest,
//! the pick moving only up and down the wheel's square; then compared.

use crate::demos::demo::{Demo, Drawn, Layout, Marker, On, Script, band, rect};
use crate::kinds::scales::scale::{ROW, ScaleTone, ValueScale, halving};
use crate::maths::colour::hsv;
use crate::maths::path::Point;
use crate::sessions::plan::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
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

/// A value scale of `cells` cells (the scale in hand's count; never its colours), painted in
/// V大's order of halving.
pub(crate) fn scale(cells: usize) -> Demo {
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
