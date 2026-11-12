//! Value masses' demonstration (`demo.rs`, S96): V大's way on a landscape of known masses made
//! without a pack (`masses.rs`), beside its picture, both frames the picture's shape as the
//! exercise's are (S99). Squinting, the frame filled with the middle grey first, the overall tone
//! (S100: Fill frame before the pens), then the darkest big masses painted in their grey, then the
//! light ones; Compare lays the answer's greys faint over the copy, its mass edges red, and hatches
//! in the answer's grey where the hand's copy strayed over more than the grey pen's width (the dark
//! masses painted wide on their right).

use crate::compositions::masses::Masses;
use crate::compositions::value_masses::{Value, ValueMasses};
use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script};
use crate::kinds::value_masses::value_masses_read::{HATCH_GAP, VALUE_PEN, differences, hatched};
use crate::maths::path::Point;
use crate::sheets::exercise::Family;
use crate::sheets::layout::PictureFit;

/// The example: a landscape four by three, its picture this wide.
const SEED: u64 = 0xDE30_5EED_0096;
const ASPECT: f64 = 0.75;
const WIDTH: u32 = 400;
/// How far the hand's dark masses run past the answer's on their right, cells (of about 222 a
/// frame unit: half as wide again as the grey pen, so Compare hatches it).
const STRAY: usize = 20;

/// Value masses.
pub(crate) fn value_masses() -> Demo {
    let masses = Masses::generate(SEED, ASPECT);
    let (size, pixels) = masses.picture(WIDTH);
    let answer = ValueMasses::of(size, &pixels);
    let copy = strayed(&answer);
    let rect = [[0.0, 0.0], PictureFit::of(size).size];
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, Drawn::Masses(rect));
    // Fill frame with the middle, the overall tone; then the darkest masses, then the light.
    let [[left, top], [right, bottom]] = rect;
    let frame = vec![
        [[left, top], [right, top]],
        [[left, bottom], [right, bottom]],
    ];
    let middle = Drawn::Band {
        rows: frame,
        colour: Some(Value::Middle.grey()),
    };
    script.step();
    script.draw(On::Copy, Marker::Ink, middle);
    for shown in [[true, false, false], [true, false, true]] {
        script.step();
        let values = Drawn::Values {
            rect,
            copy: true,
            shown,
        };
        script.draw(On::Copy, Marker::Ink, values);
    }
    let compared = compared(&answer, &copy, rect);
    let mut demo = script.compare(Family::ValueMasses, DemoLayout::Pair, compared);
    demo.view = Some(rect);
    demo.masses = Some(Box::new(masses));
    demo.values = Some(Box::new([answer, copy]));
    demo
}

/// The answer's dark masses run `STRAY` cells further on their right, as a hand paints them.
fn strayed(answer: &ValueMasses) -> ValueMasses {
    let [w, h] = answer.size.map(|side| side as usize);
    let dark = Value::Dark.index();
    let values = (0..w * h)
        .map(|at| {
            let (x, y) = (at % w, at / w);
            let near =
                (x.saturating_sub(STRAY)..=x).any(|from| answer.values[y * w + from] == dark);
            if near { dark } else { answer.values[at] }
        })
        .collect();
    ValueMasses {
        size: answer.size,
        values,
    }
}

/// Compare's marks: the answer's greys faint over the copy, its edges red, and the hatching where
/// `copy` differs from it over more than the grey pen's width, all in the picture's `rect` of the
/// frame.
fn compared(
    answer: &ValueMasses,
    copy: &ValueMasses,
    rect: [Point; 2],
) -> Vec<(On, Marker, Drawn)> {
    let [[left, top], [right, bottom]] = rect;
    let at = |[u, v]: Point| [left + u * (right - left), top + v * (bottom - top)];
    let shown = Drawn::Values {
        rect,
        copy: false,
        shown: [true; 3],
    };
    let mut marks = vec![(On::Copy, Marker::Faint, shown)];
    for edge in answer.edges() {
        let line = edge.into_iter().map(at).collect();
        marks.push((On::Copy, Marker::Answer, Drawn::Line(line)));
    }
    let size = answer.size.map(|side| side as usize);
    let values = |map: &ValueMasses| -> Vec<Value> {
        (map.values.iter())
            .map(|v| Value::of_index(*v).unwrap_or(Value::Middle))
            .collect()
    };
    let painted: Vec<Option<Value>> = values(copy).into_iter().map(Some).collect();
    let truth = values(answer);
    // A frame unit holds this many cells: the pen and the hatching's gap in cells.
    let cells = size[0] as f64 / (right - left);
    let opened = differences(&painted, &truth, size, VALUE_PEN / 2.0 * cells);
    let cell = |[x, y]: Point| [x / size[0] as f64, y / size[1] as f64];
    for (value, [a, b]) in hatched(&opened, &truth, size, HATCH_GAP * cells) {
        let line = vec![at(cell(a)), at(cell(b))];
        marks.push((On::Copy, Marker::Hatch(value), Drawn::Line(line)));
    }
    marks
}
