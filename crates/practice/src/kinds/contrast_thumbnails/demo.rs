//! A contrast thumbnail's demonstration (`demos/demo.rs`, S154), on the landscape value masses
//! show (`masses.rs`), alone in its one frame of the picture's shape: its mass edges printed and
//! the ring at its own focus given; the frame filled with the middle grey first, then at the ring
//! the dark against the light while elsewhere the dark masses are kept middle, so the rest steps
//! one value at most; Compare veils what is weaker, the ring's place left clear, the ring over it.

use super::rings::{Emphasis, Ring, rings};
use crate::compositions::levels::Level;
use crate::compositions::masses::Masses;
use crate::compositions::value_masses::{Value, ValueMasses};
use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, band, rect};
use crate::maths::path::Point;
use crate::sheets::layout::PictureFit;

/// The example: value masses' landscape, four by three, its picture this wide.
const SEED: u64 = 0xDE30_5EED_0096;
const ASPECT: f64 = 0.75;
const WIDTH: u32 = 400;
/// The veil's paper.
const PAPER: [u8; 3] = [255, 255, 255];

/// A contrast thumbnail.
pub(crate) fn thumbnail() -> Demo {
    let masses = Masses::generate(SEED, ASPECT);
    let (size, pixels) = masses.picture(WIDTH);
    let answer = ValueMasses::of(size, &pixels);
    let extent = PictureFit::of(size).size;
    let frame = [[0.0, 0.0], extent];
    let ring = rings(&answer, extent, Emphasis::Own, SEED)[0];
    let copy = planned(&answer, extent, &ring);
    // The print: the shapes and the ring, given, and over the paint again as the sheet lays them.
    let mut printed: Vec<(On, Marker, Drawn)> = (answer.edges().into_iter())
        .map(|edge| {
            let line = edge.iter().map(|[u, v]| [u * extent[0], v * extent[1]]);
            (On::Copy, Marker::Printed, Drawn::Line(line.collect()))
        })
        .collect();
    printed.push((
        On::Copy,
        Marker::Level(Level::Main),
        Drawn::Line(ring.line()),
    ));
    let mut script = Script::new();
    for (on, pen, drawn) in printed.clone() {
        script.given(on, pen, drawn);
    }
    script.step();
    let middle = band(rect(frame[0], frame[1]), Value::Middle.grey());
    let over = |paint: Drawn| {
        let mut marks = vec![(On::Copy, Marker::Ink, paint)];
        marks.extend(printed.iter().cloned());
        marks
    };
    script.together(over(middle));
    for shown in [[true, false, false], [true, false, true]] {
        script.step();
        let values = Drawn::Values {
            rect: frame,
            copy: true,
            shown,
        };
        script.together(over(values));
    }
    // The ring over the veil, as the sheet lays it again over the copy.
    let mut compared = veiled(extent, &ring);
    let laid = (
        On::Copy,
        Marker::Level(Level::Main),
        Drawn::Line(ring.line()),
    );
    compared.push(laid);
    let mut demo = script.compare(DemoLayout::Single, compared);
    demo.view = Some(frame);
    demo.values = Some(Box::new([answer, copy]));
    demo
}

/// The thumbnail a hand plans on `answer`'s shapes: in the ring the dark against the light (its
/// middle masses light), elsewhere its dark masses middle, so nothing there steps two values.
fn planned(answer: &ValueMasses, extent: Point, ring: &Ring) -> ValueMasses {
    let [w, h] = answer.size.map(|side| side as usize);
    let values = (0..w * h)
        .map(|at| {
            let p = [
                ((at % w) as f64 + 0.5) / w as f64 * extent[0],
                ((at / w) as f64 + 0.5) / h as f64 * extent[1],
            ];
            let value = Value::of_index(answer.values[at]).unwrap_or(Value::Middle);
            let planned = match (ring.holds(p), value) {
                (true, Value::Middle) => Value::Light,
                (false, Value::Dark) => Value::Middle,
                (_, kept) => kept,
            };
            planned.index()
        })
        .collect();
    ValueMasses {
        size: answer.size,
        values,
    }
}

/// Compare's veil: paper laid faint over the frame but a square round the ring.
fn veiled(extent: Point, ring: &Ring) -> Vec<(On, Marker, Drawn)> {
    let [x, y] = ring.centre;
    let r = ring.radius;
    let [w, h] = extent;
    let (left, right, top, bottom) = (x - r, x + r, (y - r).max(0.0), (y + r).min(h));
    [
        rect([0.0, 0.0], [w, top]),
        rect([0.0, bottom], [w, h]),
        rect([0.0, top], [left.max(0.0), bottom]),
        rect([right.min(w), top], [w, bottom]),
    ]
    .into_iter()
    .map(|rows| (On::Copy, Marker::Faint, band(rows, PAPER)))
    .collect()
}
