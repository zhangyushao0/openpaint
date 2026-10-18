//! `canvas-check <name>|--all` (S62): what `export-check` states of exports, for the canvas.
//! Runs scenarios as `scenario` does (on a fresh library under `artifacts/canvas-check/<name>/`,
//! writing nothing else), each canvas checkpoint beside the evaluator's image of what the canvas
//! shows there (`run_compared`): the canvas's hash, the hash the scenario expects, the evaluator's
//! image's hash, and how far the canvas lies from that image (`image_difference`: the largest and
//! mean difference, the pixels off by more than one 8-bit level and of them those away from
//! antialiased edges). A checkpoint whose expected hash is the evaluator's image's (what the canvas
//! was before S62, or S65 for the kinds routed to it) and whose canvas moved is stated so; one
//! taken while a stroke is drawn is not compared (the evaluator's image holds no stroke drawn).

use std::fs;

use openpaint_devtools::{Checkpoint, Scenario, is_file_name};
use openpaint_render::image_difference;

use crate::error::HarnessError;
use crate::launch::{fresh_library, text_fonts};
use crate::output::workspace_root;
use crate::scenario::scenario_names;

pub fn run(name: Option<&str>) -> Result<(), HarnessError> {
    let fonts = text_fonts();
    let names = match name {
        Some(name) if is_file_name(name) => vec![name.to_owned()],
        Some(_) => {
            return Err(HarnessError::Missing(
                "scenario names use a-z, 0-9 and '-' only",
            ));
        }
        None => scenario_names()?,
    };
    let root = workspace_root();
    let (mut moved, mut beyond) = (0, 0);
    for name in &names {
        let file = root
            .join("tests")
            .join("scenarios")
            .join(format!("{name}.json"));
        let scenario = Scenario::parse(&fs::read_to_string(&file)?)?;
        let dir = root.join("artifacts").join("canvas-check").join(name);
        let library = fresh_library(dir.join("library"))?;
        let report = openpaint_devtools::run_compared(&scenario, &library, &fonts)?;
        for checkpoint in report.checkpoints() {
            let (line, [shift, far]) = stated(checkpoint)?;
            println!("{name} {line}");
            moved += usize::from(shift);
            beyond += usize::from(far);
        }
    }
    println!(
        "{} scenarios: {moved} checkpoints moved from the evaluator's image, {beyond} with a \
         pixel more than a level off away from edges",
        names.len()
    );
    Ok(())
}

/// One checkpoint's line, whether its canvas moved from the hash expected, and whether a pixel
/// lies more than a level from the evaluator's away from antialiased edges.
fn stated(checkpoint: &Checkpoint) -> Result<(String, [bool; 2]), HarnessError> {
    let (name, actual) = (&checkpoint.name, checkpoint.actual());
    let expected = checkpoint.expected;
    let shift = actual != expected;
    let state = if shift { "moved" } else { "unchanged" };
    let head = format!("{name}: canvas {actual:016x} ({state}; expected {expected:016x})");
    let Some(evaluated) = &checkpoint.evaluated else {
        return Ok((format!("{head}; not compared"), [shift, false]));
    };
    let found = image_difference(evaluated, &checkpoint.canvas)
        .ok_or(HarnessError::Missing("the images differ in size"))?;
    let theirs = evaluated.content_hash();
    let was = if theirs == expected {
        " (the hash expected)"
    } else {
        ""
    };
    let line = format!(
        "{head}; evaluated {theirs:016x}{was}: largest {:?}, mean {:.2e}, off by more than one \
         level {} ({} away from edges, largest there {})",
        found.largest,
        found.mean.iter().sum::<f64>() / 4.0,
        found.beyond_one,
        found.beyond_one_inside,
        found.largest_inside
    );
    Ok((line, [shift, found.beyond_one_inside > 0]))
}
