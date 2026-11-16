//! Week 3's solids' demonstrations (`demo.rs`, S93), V大's way in his order on a box drawn whole: a
//! cylinder (an end's diagonals; through their crossing to the middles; the ellipse through them;
//! the other end alike; two lines touching both), a cone (its base alike; the opposite face's
//! diagonals for the point; two lines from it touching the base) and a sphere (its box halved in
//! height for the middle section; that section's diagonals and middles; its ellipse; a circle
//! through the ends of the ellipse's long axis), each in its one frame (S98: the box printed
//! grey), then compared as the exercise compares it (S145): the exact solid red over the hand's
//! ink (its hidden part dashed), for the eye (a cylinder's far end drawn a little narrow, a cone's
//! second side drawn past its point, V大's sphere circle near). The lines are `SolidTask`'s.

use crate::demos::demo::{
    DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, clearest, line, wobbled,
};
use crate::geometry::ellipse::{Ellipse, EllipseGuide};
use crate::kinds::solids::solid::{Pose, Solid, SolidTask, diagonals, middle_lines};
use crate::maths::path::Point;

/// The examples' seeds: a cylinder's, a cone's, a sphere's.
const SEEDS: [u64; 3] = [0xDE30_5EED_0012, 0xDE30_5EED_0013, 0xDE30_5EED_0014];
/// Seeds of the sequence tried for the clearest example.
const TRIES: u64 = 32;
/// An ellipse's outline in pieces; how far a hand's ellipse and a side stray (frame units).
const PIECES: usize = 96;
const STRAY: f64 = 0.006;
const OFF: f64 = 0.004;

/// A solid's demonstration.
pub(crate) fn solid(solid: Solid) -> Option<Demo> {
    let seed = match solid {
        Solid::Cylinder => SEEDS[0],
        Solid::Cone => SEEDS[1],
        Solid::Sphere => SEEDS[2],
    };
    // Upright and seen from above, its ellipses about half as deep as wide, as V大 first draws
    // it, the box about as wide as the card (a sphere's circle is near on every cube laid).
    let task = clearest(
        (seed, TRIES),
        |seed| SolidTask::generate(solid, seed),
        score,
    );
    let mut script = Script::new();
    given_box(&mut script, &task);
    match solid {
        Solid::Cylinder => {
            for (index, square) in task.squares.iter().enumerate() {
                if index == 0 {
                    script.step();
                    construct(&mut script, &[diagonals(&square.corners)]);
                    script.step();
                    construct(&mut script, &[middle_lines(square)]);
                    middles(&mut script, square);
                    script.step();
                } else {
                    script.step();
                    construct(
                        &mut script,
                        &[diagonals(&square.corners), middle_lines(square)],
                    );
                    middles(&mut script, square);
                }
                // The far end a little narrow: Compare's red shows it apart from the ink.
                let exact = square.exact;
                let narrow = Ellipse {
                    axes: [exact.axes[0] - DRAWN_OFF, exact.axes[1]],
                    ..exact
                };
                let shown = if index == 0 { exact } else { narrow };
                ink_ellipse(&mut script, &shown, index);
            }
            script.step();
            ink_sides(&mut script, &task.answer.sides, false);
        }
        Solid::Cone => {
            let base = task.squares.first()?;
            script.step();
            construct(&mut script, &[diagonals(&base.corners), middle_lines(base)]);
            middles(&mut script, base);
            ink_ellipse(&mut script, &base.exact, 0);
            script.step();
            construct(&mut script, &[diagonals(&task.top?)]);
            script.draw(On::Copy, Marker::Pencil, Drawn::Dot(task.point?));
            script.step();
            ink_sides(&mut script, &task.answer.sides, true);
        }
        Solid::Sphere => sphere(&mut script, &task)?,
    }
    Some(compared(script, &task))
}

/// How clear `task` is as an example on the card (`clearest`).
fn score(task: &SolidTask) -> f64 {
    let flag = |held: bool| if held { 1.0 } else { 0.0 };
    let c = &task.seen.corners;
    let above = c.iter().all(|p| p[1] > task.seen.horizon);
    let span = |k: usize| {
        let (low, high) = (c.iter()).fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| {
            (l.min(p[k]), h.max(p[k]))
        });
        (high - low).max(1e-3)
    };
    let wide = (span(0) / span(1) / 1.3).ln().abs();
    let round = task.squares.first().map_or(0.0, |s| s.exact.roundness());
    flag(above) + flag(task.pose == Pose::Upright) - (round - 0.4).abs() - wide
}

/// A sphere's steps: the box halved in height, the section's diagonals and middles, its ellipse,
/// the circle through the ends of its long axis.
fn sphere(script: &mut Script, task: &SolidTask) -> Option<()> {
    let section = task.squares.first()?;
    let halving = task.halving();
    script.step();
    let (faces, round) = halving.split_at(halving.len().saturating_sub(4));
    let pencil = |lines: &[[Point; 2]]| {
        (lines.iter())
            .map(|[a, b]| (On::Copy, Marker::Pencil, line(*a, *b)))
            .collect::<Vec<_>>()
    };
    script.together(pencil(faces));
    script.together(pencil(&round[..2]));
    script.together(pencil(&round[2..]));
    script.step();
    construct(
        script,
        &[diagonals(&section.corners), middle_lines(section)],
    );
    middles(script, section);
    script.step();
    let exact = section.exact;
    script.draw(On::Copy, Marker::Pencil, Drawn::Line(exact.outline(PIECES)));
    script.step();
    let (centre, radius) = task.circle()?;
    let ends = [0.0, std::f64::consts::PI].map(|t| exact.at(t));
    script.together(
        ends.iter()
            .map(|p| (On::Copy, Marker::Pencil, Drawn::Dot(*p)))
            .collect(),
    );
    let circle = Ellipse {
        centre,
        axes: [radius; 2],
        angle: 0.0,
    };
    ink_ellipse(script, &circle, 1);
    Some(())
}

/// The box as the exercise gives it in its one frame (S98): its horizon, its edges printed grey,
/// the hidden ones dashed.
fn given_box(script: &mut Script, task: &SolidTask) {
    let height = task.seen.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for ((a, b, _), hidden) in task.seen.edges().into_iter().zip(task.seen.hidden()) {
        let pen = if hidden {
            Marker::PrintedHidden
        } else {
            Marker::Printed
        };
        script.given(On::Copy, pen, line(a, b));
    }
}

/// Construction lines with Sketch, each group at once.
fn construct(script: &mut Script, groups: &[[[Point; 2]; 2]]) {
    let lines = (groups.iter().flatten())
        .map(|[a, b]| (On::Copy, Marker::Pencil, line(*a, *b)))
        .collect();
    script.together(lines);
}

/// `square`'s four middles dotted in pencil.
fn middles(script: &mut Script, square: &EllipseGuide) {
    script.together(
        (square.touches.iter())
            .map(|p| (On::Copy, Marker::Pencil, Drawn::Dot(*p)))
            .collect(),
    );
}

/// `exact` drawn with Draw as a hand draws it, a little off.
fn ink_ellipse(script: &mut Script, exact: &Ellipse, turn: usize) {
    let drawn = wobbled(&exact.outline(PIECES), STRAY, 0.4 + turn as f64);
    script.draw(On::Copy, Marker::Ink, Drawn::Line(drawn));
}

/// `sides` ruled with Draw, a hair off (the last, when `past`, aimed `DRAWN_OFF` beside its first
/// end, a cone's point).
fn ink_sides(script: &mut Script, sides: &[[Point; 2]], past: bool) {
    for (turn, [a, b]) in sides.iter().enumerate() {
        let off = if turn % 2 == 0 { OFF } else { -OFF };
        let mut ruled = vec![[a[0] + off, a[1] - off], [b[0] - off, b[1] + off]];
        if past && turn + 1 == sides.len() {
            let long = (b[0] - a[0]).hypot(b[1] - a[1]).max(f64::MIN_POSITIVE);
            let across = [(a[1] - b[1]) / long, (b[0] - a[0]) / long];
            ruled[0] = [a[0] + DRAWN_OFF * across[0], a[1] + DRAWN_OFF * across[1]];
        }
        script.draw(On::Copy, Marker::Ink, Drawn::Line(ruled));
    }
}

/// Compare: the exact solid red, its hidden part dashed.
fn compared(script: Script, task: &SolidTask) -> Demo {
    let mut marks: Vec<(On, Marker, Drawn)> = Vec::new();
    for stretch in task.answer.outlines.iter().flatten() {
        let pen = if stretch.hidden {
            Marker::Hidden
        } else {
            Marker::Answer
        };
        marks.push((On::Copy, pen, Drawn::Line(stretch.points.clone())));
    }
    for [a, b] in &task.answer.sides {
        marks.push((On::Copy, Marker::Answer, line(*a, *b)));
    }
    script.compare(DemoLayout::Single, marks)
}
