//! Colour under light's demonstration (S154j), on an example of the exercise's own form and
//! colours (its fixed seed's) in its one frame: the form's edges, the pin and its shadow given;
//! the frame filled with the lit ground's colour; the light family mixed from the local colour
//! toward the light; the shadow family toward the sky and darker; the cast shadow from the ground
//! toward the sky; at 稳 the reflected light toward the ground's colour; then Compare: the model's
//! colours laid faint over the copy.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, band, line, rect};
use crate::kinds::light_on_form::demo::{CELLS, bands};
use crate::kinds::lit_colours::regions::Region;
use crate::kinds::lit_colours::scene::{ColourTask, LitColour};
use crate::maths::random::derived;

/// The example's seed.
const SEED: u64 = 0xDE30_5EED_0154;

/// Colour under light's demonstration for `lit`'s form and level.
pub(crate) fn colour_under_light(lit: LitColour) -> Demo {
    let task = ColourTask::generate(lit, derived(SEED, 0));
    let middle = |k: usize| (k as f64 + 0.5) / CELLS as f64;
    let regions: Vec<Option<Region>> = (0..CELLS * CELLS)
        .map(|at| {
            let shading = task
                .task
                .shading([middle(at % CELLS), middle(at / CELLS)])?;
            Some(Region::of(&shading, task.bounces()))
        })
        .collect();
    let painted = |region: Region, pen: Marker| -> Vec<(On, Marker, Drawn)> {
        let Some(colour) = task.model(region) else {
            return Vec::new();
        };
        let held: Vec<bool> = regions.iter().map(|r| *r == Some(region)).collect();
        (bands(&held, colour).into_iter())
            .map(|drawn| (On::Copy, pen, drawn))
            .collect()
    };
    let printed = printed(&task);
    let mut script = Script::new();
    for (on, pen, drawn) in printed.iter().cloned() {
        script.given(on, pen, drawn);
    }
    let step = |script: &mut Script, marks: Vec<(On, Marker, Drawn)>| {
        script.step();
        script.together(marks.into_iter().chain(printed.iter().cloned()).collect());
    };
    let ground = task.model(Region::Ground).unwrap_or([200; 3]);
    let filled = band(rect([0.0, 0.0], [1.0, 1.0]), ground);
    step(&mut script, vec![(On::Copy, Marker::Ink, filled)]);
    step(&mut script, painted(Region::Light, Marker::Ink));
    let mut shadow = painted(Region::Shadow, Marker::Ink);
    if task.bounces() {
        // The shadow family first all of a colour, its reflected light after.
        let mut reflected = painted(Region::Reflected, Marker::Ink);
        let dark = task.model(Region::Shadow).unwrap_or([60; 3]);
        for (_, _, drawn) in &mut reflected {
            if let Drawn::Band { colour, .. } = drawn {
                *colour = Some(dark);
            }
        }
        shadow.extend(reflected);
    }
    step(&mut script, shadow);
    step(&mut script, painted(Region::Cast, Marker::Ink));
    if task.bounces() {
        step(&mut script, painted(Region::Reflected, Marker::Ink));
    }
    let compared = (Region::ALL.into_iter())
        .flat_map(|region| painted(region, Marker::Faint))
        .chain(printed.iter().cloned())
        .collect();
    script.compare(DemoLayout::Single, compared)
}

/// What the exercise prints: the form's edges, the pin and its shadow, its top a dot.
fn printed(task: &ColourTask) -> Vec<(On, Marker, Drawn)> {
    let [foot, top, tip] = task.task.pinned;
    let edges = task.task.edges.iter().map(|edge| Drawn::Line(edge.clone()));
    (edges.chain([line(foot, top), line(foot, tip), Drawn::Dot(top)]))
        .map(|drawn| (On::Copy, Marker::Printed, drawn))
        .collect()
}
