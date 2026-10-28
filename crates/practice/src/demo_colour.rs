//! The colour kinds' demonstrations (`demo.rs`): a colour (the wheel's ring to the hue's family,
//! then lighter or darker, then purer or greyer, the frame filled), a colour combination (the
//! overall tone, the largest area first, each neighbour against it, the weak contrast kept weak)
//! and a value scale (the middle cell first, then the quarters, the pick moving only up and down
//! the wheel's square), each then compared.

use super::{Demo, Drawn, Layout, Marker, On, Script, clearest};
use crate::areas::{Areas, Arrangement, GRID};
use crate::colour::{Rgb8, delta_e, from_hsv, from_oklab, hsv, oklab};
use crate::colour_match::{Lightness, Purity, Tone};
use crate::path::Point;
use crate::plan::Family;
use crate::random::derived;
use crate::scale::{BARS, ROW, ScaleTone, ValueScale};

const COLOUR_SEED: u64 = 0xDE30_5EED_0004;
const AREAS_SEED: u64 = 0xDE30_5EED_0005;
const SCALE_SEED: u64 = 0xDE30_5EED_0006;
/// A value scale's example has five cells: its middle and quarters are cells.
const SCALE_CELLS: usize = 5;
/// A bar's half width under a cell, a share of the cell's width.
const BAR: f64 = 0.22;

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

/// A value scale.
pub(super) fn scale() -> Demo {
    let scale = (0..64)
        .map(|k| ValueScale::generate(ScaleTone::Hue, derived(SCALE_SEED, k)))
        .find(|scale| scale.cells == SCALE_CELLS)
        .unwrap_or_else(|| ValueScale::generate(ScaleTone::Hue, SCALE_SEED));
    let targets = scale.targets();
    let mut script = Script::new();
    scale_given(&mut script, &scale);
    // The copy's lightness per cell: the ends given, the others a little off as an eye judges.
    let mut values = targets.clone();
    values[2] += 0.008;
    values[1] -= 0.012;
    values[3] += 0.01;
    for (turn, cell) in [2, 1, 3].into_iter().enumerate() {
        if turn < 2 {
            script.step();
        }
        let colour = scale.colour(values[cell]);
        script.draw(On::Wheel, Marker::Ink, Drawn::Pick(hsv(colour)));
        let [a, b] = scale.cell(cell);
        script.draw(On::Copy, Marker::Ink, band(rect(a, b), colour));
    }
    let compared = scale_compared(&scale, &values);
    script.compare(Family::Scale, Layout::Wheel, compared)
}

/// What a value scale's exercise gives: its ends as halves in the original's frame, its cells in
/// the copy's with the ends filled, the wheel picking the dark end.
fn scale_given(script: &mut Script, scale: &ValueScale) {
    let [dark, light] = scale.ends();
    let [left, top, right, bottom] = ROW;
    let middle = (left + right) / 2.0;
    let halves = [([left, middle], dark), ([middle, right], light)];
    for ([from, to], colour) in halves {
        let half = band(rect([from, top], [to, bottom]), colour);
        script.given(On::Original, Marker::Given, half);
    }
    for cell in 0..scale.cells {
        let [a, b] = scale.cell(cell);
        let outline = vec![a, [b[0], a[1]], b, [a[0], b[1]], a];
        script.given(On::Copy, Marker::Rule, Drawn::Line(outline));
    }
    for (end, colour) in [(0, dark), (scale.cells - 1, light)] {
        let [a, b] = scale.cell(end);
        script.given(On::Copy, Marker::Given, band(rect(a, b), colour));
    }
    script.given(On::Wheel, Marker::Given, Drawn::Pick(hsv(dark)));
}

/// A value scale compared: the even scale in the original's frame, under the copy's row each
/// cell's lightness (`values`) as a bar in its colour with its even step a red tick.
fn scale_compared(scale: &ValueScale, values: &[f64]) -> Vec<(On, Marker, Drawn)> {
    let [left, _, right, _] = ROW;
    let mut compared = Vec::new();
    for (cell, target) in scale.targets().iter().enumerate() {
        let [a, b] = scale.cell(cell);
        compared.push((
            On::Original,
            Marker::Given,
            band(rect(a, b), scale.colour(*target)),
        ));
        let [[x0, _], [x1, _]] = [a, b];
        let (centre, width) = ((x0 + x1) / 2.0, x1 - x0);
        let [high, low] = BARS;
        let level = |value: f64| low - value * (low - high);
        let bar = rect(
            [centre - BAR * width, level(values[cell])],
            [centre + BAR * width, low],
        );
        compared.push((
            On::Copy,
            Marker::Given,
            band(bar, scale.colour(values[cell])),
        ));
        let tick = vec![
            [x0 + 0.1 * width, level(*target)],
            [x1 - 0.1 * width, level(*target)],
        ];
        compared.push((On::Copy, Marker::Answer, Drawn::Line(tick)));
    }
    compared.push((
        On::Copy,
        Marker::Rule,
        Drawn::Line(vec![[left, BARS[1]], [right, BARS[1]]]),
    ));
    compared
}
