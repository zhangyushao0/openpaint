//! Light on form's demonstration (S153), on an example of the exercise's own form and level (its
//! fixed seed's first) in its one frame: the form's edges, the pin and its shadow given; the way
//! the light comes, a pass in the air from the pin's top to its shadow's tip; the frame filled with
//! the light grey; the shadow family dark (the faces or the band turned away and the cast shadow);
//! at 熟练 the halftone where the light grazes; at 稳 the darkest along the core and at the
//! contact; then Compare as the exercise lays it: the answer faint over the copy, the terminator
//! and the cast shadow's outline red, hatched where the hand's dark ran past the terminator.

use crate::compositions::value_masses::{Value, ValueMasses};
use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, band, bounds, line, rect};
use crate::kinds::light_on_form::answer::{Grey, Shading};
use crate::kinds::light_on_form::scene::{LightTask, Lit, Tones};
use crate::kinds::tools::DARKEST;
use crate::kinds::value_masses::value_masses_read::{HATCH_GAP, VALUE_PEN, differences, hatched};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::maths::random::derived;

/// The example's seed; its maps' cells across the frame.
const SEED: u64 = 0xDE30_5EED_0153;
const CELLS: usize = 200;
/// How far the hand's dark runs past the answer's on the form, cells (0.07 of the frame: wider
/// than the grey pen, so Compare hatches it).
const STRAY: usize = 14;
/// Paper kept round what the example shows (frame units).
const AROUND: f64 = 0.06;

/// Light on form's demonstration for `lit`'s form and level.
pub(crate) fn light_on_form(lit: Lit) -> Demo {
    let task = LightTask::generate(lit, derived(SEED, 0));
    let map = task.map([CELLS as u32; 2]);
    let answer = values(&map);
    let copy = strayed(&answer, &map);
    let frame = [[0.0, 0.0], [1.0, 1.0]];
    let mut script = Script::new();
    let printed = printed(&task);
    for (on, pen, drawn) in printed.iter().cloned() {
        script.given(on, pen, drawn);
    }
    // Where the light comes from: along the pin's top to its shadow's tip.
    let [_, top, tip] = task.pinned;
    script.step();
    script.draw(On::Copy, Marker::Air, line(top, tip));
    // Each grey laid, what is printed lying over it as on the sheet.
    let painted = |script: &mut Script, paint: Vec<(On, Marker, Drawn)>| {
        script.step();
        script.together(paint.into_iter().chain(printed.iter().cloned()).collect());
    };
    let light = band(rect(frame[0], frame[1]), Grey::Light.grey());
    painted(&mut script, vec![(On::Copy, Marker::Ink, light)]);
    for shown in [[true, false, false], [true, true, false]] {
        if shown[1] && task.lit.tones == Tones::Two {
            continue;
        }
        let values = Drawn::Values {
            rect: frame,
            copy: true,
            shown,
        };
        painted(&mut script, vec![(On::Copy, Marker::Ink, values)]);
    }
    if task.lit.tones == Tones::Four {
        let darkest: Vec<bool> = (map.iter())
            .map(|s| s.is_some_and(|s| s.tone == Grey::Darkest))
            .collect();
        let bands = (bands(&darkest, DARKEST).into_iter())
            .map(|drawn| (On::Copy, Marker::Ink, drawn))
            .collect();
        painted(&mut script, bands);
    }
    let mut compared = compared(&task, &answer, &copy);
    compared.splice(1..1, printed.iter().cloned());
    let mut demo = script.compare(DemoLayout::Single, compared);
    demo.values = Some(Box::new([answer, copy]));
    demo.view = Some(view(&task));
    demo
}

/// What the exercise prints: the form's edges, the pin and its shadow, its top a dot.
fn printed(task: &LightTask) -> Vec<(On, Marker, Drawn)> {
    let [foot, top, tip] = task.pinned;
    let edges = task.edges.iter().map(|edge| Drawn::Line(edge.clone()));
    (edges.chain([line(foot, top), line(foot, tip), Drawn::Dot(top)]))
        .map(|drawn| (On::Copy, Marker::Printed, drawn))
        .collect()
}

/// The part of the frame the example shows: a square round the form, the pin and the cast
/// shadow, a little paper round them, inside the frame.
fn view(task: &LightTask) -> [Point; 2] {
    let parting = task.parting();
    let points: Vec<Point> = (task.edges.iter().flatten())
        .chain(parting.cast.iter().flatten())
        .chain(task.pinned.iter())
        .copied()
        .collect();
    let [low, high] = bounds(&points).unwrap_or([[0.0; 2], [1.0; 2]]);
    let side = ((high[0] - low[0]).max(high[1] - low[1]) + 2.0 * AROUND).min(1.0);
    let middle = [0, 1].map(|k| ((low[k] + high[k]) / 2.0).clamp(side / 2.0, 1.0 - side / 2.0));
    [
        [middle[0] - side / 2.0, middle[1] - side / 2.0],
        [middle[0] + side / 2.0, middle[1] + side / 2.0],
    ]
}

/// `map`'s tones as value masses' map (the darkest as the dark): its cells across the frame.
fn values(map: &[Option<Shading>]) -> ValueMasses {
    let index = |s: &Option<Shading>| match s.map_or(Grey::Light, |s| s.tone) {
        Grey::Darkest | Grey::Dark => Value::Dark.index(),
        Grey::Middle => Value::Middle.index(),
        Grey::Light => Value::Light.index(),
    };
    ValueMasses {
        size: [CELLS as u32; 2],
        values: map.iter().map(index).collect(),
    }
}

/// The answer's dark runs on the form `STRAY` cells further either side, as a hand paints them.
fn strayed(answer: &ValueMasses, map: &[Option<Shading>]) -> ValueMasses {
    let dark = Value::Dark.index();
    let form = |at: usize| map[at].is_some_and(|s| s.on_form);
    let values = (0..CELLS * CELLS)
        .map(|at| {
            let (x, y) = (at % CELLS, at / CELLS);
            let reach = x.saturating_sub(STRAY)..=(x + STRAY).min(CELLS - 1);
            let near = reach
                .into_iter()
                .any(|from| answer.values[y * CELLS + from] == dark);
            if near && form(at) {
                dark
            } else {
                answer.values[at]
            }
        })
        .collect();
    ValueMasses {
        size: answer.size,
        values,
    }
}

/// Compare's marks: the answer faint over the copy, the terminator and the cast shadow's outline
/// red, the hatching where `copy` differs from `answer` over more than the grey pen's width.
fn compared(
    task: &LightTask,
    answer: &ValueMasses,
    copy: &ValueMasses,
) -> Vec<(On, Marker, Drawn)> {
    let frame = [[0.0, 0.0], [1.0, 1.0]];
    let shown = Drawn::Values {
        rect: frame,
        copy: false,
        shown: [true; 3],
    };
    let mut marks = vec![(On::Copy, Marker::Faint, shown)];
    let parting = task.parting();
    for edge in parting.terminator.into_iter().chain(parting.cast) {
        marks.push((On::Copy, Marker::Answer, Drawn::Line(edge)));
    }
    let tones = |map: &ValueMasses| -> Vec<Value> {
        (map.values.iter())
            .map(|v| Value::of_index(*v).unwrap_or(Value::Middle))
            .collect()
    };
    let painted: Vec<Option<Value>> = tones(copy).into_iter().map(Some).collect();
    let truth = tones(answer);
    let cells = CELLS as f64;
    let opened = differences(&painted, &truth, [CELLS; 2], VALUE_PEN / 2.0 * cells);
    let at = |[x, y]: Point| [x / cells, y / cells];
    for (value, [a, b]) in hatched(&opened, &truth, [CELLS; 2], HATCH_GAP * cells) {
        marks.push((
            On::Copy,
            Marker::Hatch(value),
            Drawn::Line(vec![at(a), at(b)]),
        ));
    }
    marks
}

/// A band still open: the run (first and last cell) on its lowest row, and its rows so far.
type Band = ((usize, usize), Vec<[Point; 2]>);

/// The cells `held` (of a grid `CELLS` across the frame) as bands of `colour`: each row's runs
/// joined to the runs under them they overlap.
fn bands(held: &[bool], colour: Rgb8) -> Vec<Drawn> {
    let cell = 1.0 / CELLS as f64;
    let row = |y: f64, (from, to): (usize, usize)| {
        [
            [from as f64 * cell, y * cell],
            [(to + 1) as f64 * cell, y * cell],
        ]
    };
    let mut open: Vec<Band> = Vec::new();
    let mut done: Vec<Vec<[Point; 2]>> = Vec::new();
    for y in 0..CELLS {
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for x in 0..CELLS {
            if !held[y * CELLS + x] {
                continue;
            }
            match runs.last_mut() {
                Some((_, to)) if *to + 1 == x => *to = x,
                _ => runs.push((x, x)),
            }
        }
        let mut next = Vec::new();
        for run in runs {
            let joins = |(span, _): &Band| run.0 <= span.1 && run.1 >= span.0;
            let mut rows = match open.iter().position(joins) {
                Some(k) => open.remove(k).1,
                None => vec![row(y as f64, run)],
            };
            rows.push(row(y as f64 + 0.5, run));
            next.push((run, rows));
        }
        for (run, mut rows) in open.drain(..) {
            rows.push(row(y as f64, run));
            done.push(rows);
        }
        open = next;
    }
    done.extend(open.into_iter().map(|(run, mut rows)| {
        rows.push(row(CELLS as f64, run));
        rows
    }));
    done.into_iter().map(|rows| band(rows, colour)).collect()
}
