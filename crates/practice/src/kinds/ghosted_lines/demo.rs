//! Ghosted lines' demonstrations (S156), each on an example of its own in its one frame (its fixed
//! seed's task), Drawabox's way: superimposed lines ghosted along the printed line, then eight
//! strokes from its dot, fraying a little at the far end; a pair's two points planned, the motion
//! ghosted from the shoulder three times, one sure stroke, the other pairs alike; a plane's edges
//! from corner to corner, its diagonals, then through their crossing a line toward where each
//! pair of opposite edges meet, ghosted on toward them first. Compare lays the exact lines red.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, line, wobbled};
use crate::kinds::ghosted_lines::ghosted::{Ghosted, GhostedTask};
use crate::kinds::ghosted_lines::marks::{run_on, stubs};
use crate::maths::path::{Point, resample};
use crate::maths::random::derived;

/// The examples' seed, and how far a hand's sure stroke strays (frame units).
const SEED: u64 = 0xDE30_5EED_0156;
const STRAY: f64 = 0.004;
/// How far a pass in the air strays, and the superimposed strokes' fraying at the far end.
const AIR: f64 = 0.012;
const FRAY: f64 = 0.007;

/// `ghosted`'s demonstration.
pub(crate) fn ghosted(ghosted: Ghosted) -> Demo {
    let task = GhostedTask::generate(ghosted, derived(SEED, 0));
    match ghosted {
        Ghosted::Superimposed => superimposed(&task),
        Ghosted::Pairs => pairs(&task),
        Ghosted::Plane => plane(&task),
    }
}

/// The straight line from `a` to `b` in short steps, as a hand's stroke strays along it.
fn along([a, b]: [Point; 2]) -> Vec<Point> {
    resample(&[a, b], 0.03)
}

/// The line laid red at Compare, each of `task`'s.
fn answered(task: &GhostedTask) -> Vec<(On, Marker, Drawn)> {
    (task.lines.iter())
        .map(|[a, b]| (On::Copy, Marker::Answer, line(*a, *b)))
        .collect()
}

/// The printed line and its dot; ghosted along it twice; eight strokes from the dot, the far end
/// fraying a little either side.
fn superimposed(task: &GhostedTask) -> Demo {
    let [start, end] = task.lines[0];
    let mut script = Script::new();
    script.given(On::Copy, Marker::Printed, line(start, end));
    script.given(On::Copy, Marker::Printed, Drawn::Dot(start));
    script.step();
    for phase in [0.4, 2.1] {
        let pass = wobbled(&along([start, end]), AIR, phase);
        script.draw(On::Copy, Marker::Air, Drawn::Line(pass));
    }
    script.step();
    let long = [end[0] - start[0], end[1] - start[1]];
    let length = long[0].hypot(long[1]).max(f64::MIN_POSITIVE);
    let across = [-long[1] / length, long[0] / length];
    for k in 0..8 {
        let fray = FRAY * (f64::from(k) - 3.5) / 3.5;
        let to = [end[0] + across[0] * fray, end[1] + across[1] * fray];
        let stroke = wobbled(&along([start, to]), STRAY, 0.3 + f64::from(k));
        script.draw(On::Copy, Marker::Ink, Drawn::Line(stroke));
    }
    script.compare(DemoLayout::Single, answered(task))
}

/// The pairs' points; the first pair's two planned, ghosted three times, one sure stroke; the
/// other pairs drawn alike.
fn pairs(task: &GhostedTask) -> Demo {
    let mut script = Script::new();
    for [a, b] in task.lines.iter().flat_map(|line| stubs(*line)) {
        script.given(On::Copy, Marker::Printed, line(a, b));
    }
    for at in task.points() {
        script.given(On::Copy, Marker::Printed, Drawn::Dot(at));
    }
    let first = task.lines[0];
    script.step();
    for at in first {
        script.draw(On::Copy, Marker::Air, Drawn::Ring(at));
    }
    script.step();
    for phase in [0.3, 1.7, 3.1] {
        let pass = wobbled(&along(first), AIR, phase);
        script.draw(On::Copy, Marker::Air, Drawn::Line(pass));
    }
    script.step();
    for (k, line) in task.lines.iter().enumerate() {
        let stroke = wobbled(&along(*line), STRAY, 0.5 + k as f64);
        script.draw(On::Copy, Marker::Ink, Drawn::Line(stroke));
    }
    script.compare(DemoLayout::Single, answered(task))
}

/// The plane's corners; its edges, the first ghosted first; its diagonals; the edges and the
/// lines through the centre ghosted on toward where they meet, then the two middles.
fn plane(task: &GhostedTask) -> Demo {
    let mut script = Script::new();
    for at in task.points() {
        script.given(On::Copy, Marker::Printed, Drawn::Dot(at));
    }
    let Some(plane) = task.plane else {
        return script.compare(DemoLayout::Single, answered(task));
    };
    let inked = |script: &mut Script, lines: &[[Point; 2]], from: f64| {
        for (k, line) in lines.iter().enumerate() {
            let stroke = wobbled(&along(*line), STRAY, from + k as f64);
            script.draw(On::Copy, Marker::Ink, Drawn::Line(stroke));
        }
    };
    let edges = plane.edges();
    script.step();
    script.draw(On::Copy, Marker::Air, Drawn::Line(along(edges[0])));
    inked(&mut script, &edges, 0.4);
    script.step();
    let [a, b, c, d] = plane.corners;
    inked(&mut script, &[[a, c], [b, d]], 1.3);
    script.step();
    let ghosted = (run_on(&plane).into_iter())
        .map(|[from, to]| (On::Copy, Marker::Air, line(from, to)))
        .collect();
    script.together(ghosted);
    inked(&mut script, &plane.middles(), 2.2);
    script.compare(DemoLayout::Single, answered(task))
}
