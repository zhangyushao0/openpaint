//! V大's five tricks' demonstrations (`demos/demo.rs`, stage 3), each in its one frame (S98): the
//! box printed grey, the trick's construction in order with Sketch, the answer with Draw, the
//! example framed on the shaded face; then compared as the exercise compares it: the answer red
//! and only what is off (`SpaceCheck`'s own marks on the hand's ink, which lands a little off on
//! purpose). The tricks' lines are `SpaceTask::construction`'s and their answers
//! `SpaceTask::answer`'s.

use crate::demos::demo::{
    DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, bounds, clearest, line, off_marks, tick,
};
use crate::demos::demo_points::{THIRD_TICK, crossed, moved, read};
use crate::kinds::points_in_space::space::{SpaceAnswer, SpaceTask, Trick};
use crate::kinds::points_in_space::space_check::SpaceCheck;
use crate::maths::path::Point;

/// The examples' seeds: Centre's, Halves', Thirds', Next face's, Mirror's.
const SEEDS: [u64; 5] = [
    0xDE30_5EED_000D,
    0xDE30_5EED_000E,
    0xDE30_5EED_000F,
    0xDE30_5EED_0010,
    0xDE30_5EED_0011,
];

/// A point in space by one of V大's tricks.
pub(crate) fn trick(trick: Trick) -> Option<Demo> {
    // The construction's lines by step; the answer's step draws the last ones listed with it.
    let (seed, steps): (u64, &[&[usize]]) = match trick {
        Trick::Centre => (SEEDS[0], &[&[0, 1], &[]]),
        Trick::Halves => (SEEDS[1], &[&[0, 1], &[2], &[]]),
        Trick::Thirds => (SEEDS[2], &[&[], &[0, 1], &[2], &[]]),
        Trick::Step => (SEEDS[3], &[&[0], &[2, 3], &[1], &[]]),
        Trick::Mirror => (SEEDS[4], &[&[0, 1, 2], &[3, 4], &[5, 6, 7], &[8], &[9]]),
    };
    // The face as wide as the card's picture is (about twice as wide as tall), so framed on it
    // its construction reads apart; a mirror's dot well off the middle and high, so its rectangle
    // is wide and tall.
    let task = clearest(
        (seed, 96),
        |seed| SpaceTask::generate(trick, seed),
        |task| {
            let wide = bounds(&task.corners).map_or(0.0, |[low, high]| {
                ((high[0] - low[0]) / (high[1] - low[1]).max(1e-3)).min(2.0)
            });
            let room = task.given.map_or(0.0, |[u, v]| (0.5 - u).abs() + v / 2.0);
            wide + room
        },
    );
    let mut script = Script::new();
    given_box(&mut script, &task);
    let construction = task.construction();
    for (index, lines) in steps.iter().enumerate() {
        script.step();
        if trick == Trick::Thirds && index == 0 {
            // The near upright edge shows true: divided in three.
            for v in [1.0 / 3.0, 2.0 / 3.0] {
                let mark = tick(task.at([0.0, v]), task.at([0.0, 0.0]), THIRD_TICK);
                script.draw(On::Copy, Marker::Pencil, Drawn::Line(mark));
            }
        }
        for at in *lines {
            let [a, b] = construction[*at];
            script.draw(On::Copy, Marker::Pencil, line(a, b));
        }
    }
    // The answer with Draw, after the last step's lines.
    let (mine, compared) = answered(&task);
    for drawn in mine {
        script.draw(On::Copy, Marker::Ink, drawn);
    }
    let mut demo = script.compare(DemoLayout::Single, compared);
    // Framed on the shaded face (and a next face beyond it), so every construction line and the
    // answer read apart; the rest of the box and the lines to its vanishing point leave the
    // picture at its edge.
    let mut held = task.corners.to_vec();
    if let SpaceAnswer::Lines(lines) = task.answer() {
        held.extend(lines.into_iter().flatten());
    }
    demo.view = bounds(&held);
    Some(demo)
}

/// The box of `task` as the exercise gives it in its one frame, printed grey (S98): its face
/// shaded, the horizon, the edges the eye sees, a mirror's dot.
fn given_box(script: &mut Script, task: &SpaceTask) {
    let [near_bottom, far_bottom, far_top, near_top] = task.corners;
    let face = Drawn::Band {
        rows: vec![[near_top, far_top], [near_bottom, far_bottom]],
        colour: None,
    };
    script.given(On::Copy, Marker::Printed, face);
    let height = task.seen.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for ((a, b, _), hidden) in task.seen.edges().into_iter().zip(task.seen.hidden()) {
        if !hidden {
            script.given(On::Copy, Marker::Printed, line(a, b));
        }
    }
    if let Some(dot) = task.given_at() {
        script.given(On::Copy, Marker::Printed, Drawn::Dot(dot));
    }
}

/// What a hand draws for `task`'s answer, a part of it off by `DRAWN_OFF` (a point beside it; the
/// halving line through the vanishing point but above the centre; the far third's upright, or
/// the next face's, beside its place), and Compare's marks: the exact answer red, and what the
/// exercise's check finds off.
fn answered(task: &SpaceTask) -> (Vec<Drawn>, Vec<(On, Marker, Drawn)>) {
    let (mine, strokes, mut compared): (Vec<Drawn>, Vec<Vec<Point>>, Vec<_>) = match task.answer() {
        SpaceAnswer::Point(p) => {
            let mark = moved(p, [0.8 * DRAWN_OFF, -0.6 * DRAWN_OFF]);
            let compared = vec![(On::Copy, Marker::Answer, Drawn::Cross(p))];
            (vec![Drawn::Cross(mark)], crossed(mark).to_vec(), compared)
        }
        SpaceAnswer::Lines(lines) => {
            let shift = [DRAWN_OFF, 0.0];
            let mine: Vec<[Point; 2]> = match task.trick {
                // A line to the vanishing point a tenth of the face above its middle.
                Trick::Halves => vec![[task.at([0.0, 0.6]), task.at([1.0, 0.6])]],
                // The next face's answer is the whole face; the hand draws its far upright alone.
                Trick::Step => vec![lines[1].map(|p| moved(p, shift))],
                _ => (lines.iter().enumerate())
                    .map(|(k, [a, b])| {
                        let by = if k + 1 == lines.len() {
                            shift
                        } else {
                            [0.0; 2]
                        };
                        [moved(*a, by), moved(*b, by)]
                    })
                    .collect(),
            };
            let compared = (lines.iter())
                .map(|[a, b]| (On::Copy, Marker::Answer, line(*a, *b)))
                .collect();
            let drawn = mine.iter().map(|[a, b]| line(*a, *b)).collect();
            (
                drawn,
                mine.iter().map(|ends| ends.to_vec()).collect(),
                compared,
            )
        }
    };
    compared.extend(off_marks(&SpaceCheck::of(task, &read(&strokes)).off));
    (mine, compared)
}
