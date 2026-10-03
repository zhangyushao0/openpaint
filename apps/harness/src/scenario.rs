//! `scenario <name>`: runs `tests/scenarios/<name>.json` through the product frame loop on a
//! fresh library in `artifacts/scenarios/<name>/library/` (which `restart` reopens) and writes
//! what it saw under `artifacts/scenarios/<name>/`:
//!
//! - `<screenshot>.png`: the screen at each screenshot step, for looking at;
//! - `<checkpoint>.canvas.png`: the canvas each time its checkpoint passes, the reference for
//!   later diffs (a pass also removes the failure files below);
//! - on a failed checkpoint, `<checkpoint>.actual.png` and `<checkpoint>.diff.png` (pixels that
//!   differ from the last passing canvas in red), with their paths printed;
//! - `<export>.export.png` or `.export.jpg`: each exported file as the app wrote it.

use std::fs;
use std::path::Path;

use openpaint_devtools::{Checkpoint, Export, Record, Scenario, diff, is_file_name};

use crate::error::HarnessError;
use crate::launch::fresh_library;
use crate::output::{read_rgba, workspace_root, write_rgba};

pub fn run(name: &str) -> Result<(), HarnessError> {
    if !is_file_name(name) {
        return Err(HarnessError::Missing(
            "scenario names use a-z, 0-9 and '-' only",
        ));
    }
    let root = workspace_root();
    let file = root
        .join("tests")
        .join("scenarios")
        .join(format!("{name}.json"));
    println!("scenario {}", file.display());
    let scenario = Scenario::parse(&fs::read_to_string(&file)?)?;
    let dir = root.join("artifacts").join("scenarios").join(name);
    let library = fresh_library(dir.join("library"))?;
    let report = openpaint_devtools::run(&scenario, &library)?;
    println!("adapter {}", report.adapter);
    fs::create_dir_all(&dir)?;
    for record in &report.records {
        match record {
            Record::Checkpoint(checkpoint) => write_checkpoint(&dir, checkpoint)?,
            Record::Screenshot(shot) => {
                let path = dir.join(format!("{}.png", shot.name));
                write_rgba(&path, shot.screen.size(), shot.screen.pixels())?;
                println!("screenshot {}: {}", shot.name, path.display());
            }
            Record::Export(export) => write_export(&dir, export)?,
            Record::View(view) => {
                let (zoom, rotation, pan) = view.actual;
                println!(
                    "view {}: {}, zoom {zoom:.3}, rotation {rotation:.1}°, pan [{:.1}, {:.1}] \
                     (expected {:?})",
                    view.name,
                    if view.passed() { "pass" } else { "FAIL" },
                    pan[0],
                    pan[1],
                    view.expected
                );
            }
        }
    }
    let total = report.checkpoints().count() + report.exports().count() + report.views().count();
    let failed = report.checkpoints().filter(|c| !c.passed()).count()
        + report.exports().filter(|e| !e.passed()).count()
        + report.views().filter(|v| !v.passed()).count();
    println!(
        "scenario {name}: {} of {total} checkpoints, exports and views passed, {} frames, {:.2} s",
        total - failed,
        report.frames,
        report.elapsed.as_secs_f64()
    );
    if failed > 0 {
        return Err(HarnessError::Checkpoints { failed, total });
    }
    Ok(())
}

fn write_export(dir: &Path, export: &Export) -> Result<(), HarnessError> {
    let path = dir.join(format!(
        "{}.export.{}",
        export.name,
        export.format.extension()
    ));
    fs::write(&path, &export.bytes)?;
    let [width, height] = export.size;
    println!(
        "export {}: {}, {} {width} x {height}, {} bytes, largest difference from the canvas {}, \
         mean {:.3}: {}",
        export.name,
        if export.passed() { "pass" } else { "FAIL" },
        export.format.name(),
        export.bytes.len(),
        export.largest_difference,
        export.mean_difference,
        path.display()
    );
    Ok(())
}

fn write_checkpoint(dir: &Path, checkpoint: &Checkpoint) -> Result<(), HarnessError> {
    let name = &checkpoint.name;
    let canvas = &checkpoint.canvas;
    let reference = dir.join(format!("{name}.canvas.png"));
    if checkpoint.passed() {
        write_rgba(&reference, canvas.size(), canvas.pixels())?;
        for stale in ["actual", "diff"] {
            // Left by an earlier failure; absent is the normal case.
            let _ = fs::remove_file(dir.join(format!("{name}.{stale}.png")));
        }
        println!(
            "checkpoint {name}: pass, canvas {:016x}",
            checkpoint.actual()
        );
        return Ok(());
    }
    println!(
        "checkpoint {name}: FAIL, canvas {:016x}, expected {:016x}",
        checkpoint.actual(),
        checkpoint.expected
    );
    let actual = dir.join(format!("{name}.actual.png"));
    write_rgba(&actual, canvas.size(), canvas.pixels())?;
    println!("  actual canvas: {}", actual.display());
    let marked = read_rgba(&reference).and_then(|(size, pixels)| diff(size, &pixels, canvas));
    let Some(marked) = marked else {
        println!(
            "  no diff: no passing canvas of this size at {}",
            reference.display()
        );
        return Ok(());
    };
    let path = dir.join(format!("{name}.diff.png"));
    write_rgba(&path, marked.size, &marked.pixels)?;
    println!(
        "  diff against the last passing canvas, {} pixels differ (red): {}",
        marked.differing,
        path.display()
    );
    if marked.differing == 0 {
        println!("  the pixels are the last passing ones; the expected hash changed");
    }
    Ok(())
}
