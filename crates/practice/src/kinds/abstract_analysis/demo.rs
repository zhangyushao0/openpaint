//! Abstract analysis's demonstration (`demos/demo.rs`, S112), on the landscape the divisions share
//! (`demos/demo_landscape.rs`), the picture lying in its one frame: the house, the place that draws
//! the eye most, ringed red (强); the tree ringed green (次) and the calm sky hatched blue (弱);
//! Compare lays the picture faint, as a veil does, the marks over it.

use crate::compositions::levels::Level;
use crate::compositions::masses::Paint;
use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, wobbled};
use crate::demos::demo_landscape::{Landscape, STRAY};
use crate::maths::path::Point;

/// A ring's reach past what it rings, a share of its half extent.
const RING: f64 = 1.35;
/// The hatching's strokes and how far apart they lie, frame units.
const HATCHES: usize = 5;
const HATCH_GAP: f64 = 0.035;

/// An abstract analysis.
pub(crate) fn analysis() -> Demo {
    let example = Landscape::new();
    let mut script = Script::new();
    script.given(On::Copy, Marker::Given, Drawn::Masses(example.rect()));
    script.step();
    let strong = ring(&example, Paint::Wall);
    script.draw(
        On::Copy,
        Marker::Level(Level::Main),
        Drawn::Line(strong.clone()),
    );
    script.step();
    let second = ring(&example, Paint::Crown);
    script.draw(
        On::Copy,
        Marker::Level(Level::Second),
        Drawn::Line(second.clone()),
    );
    let hatching = hatching(&example);
    let weak = |line: &Vec<Point>| {
        (
            On::Copy,
            Marker::Level(Level::Third),
            Drawn::Line(line.clone()),
        )
    };
    script.together(hatching.iter().map(weak).collect());
    let mut compared = vec![
        example.laid(Marker::Faint),
        (On::Copy, Marker::Level(Level::Main), Drawn::Line(strong)),
        (On::Copy, Marker::Level(Level::Second), Drawn::Line(second)),
    ];
    compared.extend(hatching.iter().map(weak));
    let mut demo = script.compare(DemoLayout::Single, compared);
    demo.view = Some(example.rect());
    demo.masses = Some(Box::new(example.masses));
    demo
}

/// The extent of the parts painted `paint` (frame units): their top left and bottom right.
fn extent(example: &Landscape, paint: Paint) -> [Point; 2] {
    let points = (example.masses.parts.iter())
        .filter(|part| part.paint == paint)
        .flat_map(|part| part.outline.iter().map(|p| example.at(*p)));
    points.fold(
        [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]],
        |[low, high], [x, y]| {
            [
                [low[0].min(x), low[1].min(y)],
                [high[0].max(x), high[1].max(y)],
            ]
        },
    )
}

/// A hand's ring round the parts painted `paint`.
fn ring(example: &Landscape, paint: Paint) -> Vec<Point> {
    let [low, high] = extent(example, paint);
    let centre = [
        f64::midpoint(low[0], high[0]),
        f64::midpoint(low[1], high[1]),
    ];
    let reach = [high[0] - low[0], high[1] - low[1]].map(|side| side / 2.0 * RING);
    let circle: Vec<Point> = (0..=32)
        .map(|k| {
            let turn = std::f64::consts::TAU * f64::from(k) / 32.0;
            [
                centre[0] + reach[0] * turn.cos(),
                centre[1] + reach[1] * turn.sin(),
            ]
        })
        .collect();
    wobbled(&circle, STRAY, 0.7)
}

/// A few slanted strokes in the calm sky, at the picture's top right.
fn hatching(example: &Landscape) -> Vec<Vec<Point>> {
    let [low, high] = extent(example, Paint::Sky);
    let [_, picture] = example.rect();
    let [top, bottom] = [low[1].max(0.0), high[1].min(picture[1])];
    let height = ((bottom - top) / 3.0).min(HATCH_GAP * HATCHES as f64);
    let right = high[0].min(picture[0]) - HATCH_GAP;
    let left = right - height - HATCH_GAP * (HATCHES - 1) as f64;
    let top = top + HATCH_GAP;
    (0..HATCHES)
        .map(|k| {
            let x = left + HATCH_GAP * k as f64;
            vec![[x, top + height], [x + height, top]]
        })
        .collect()
}
