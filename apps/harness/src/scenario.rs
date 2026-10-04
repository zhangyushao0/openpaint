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
//!
//! `scenario --all` runs every `tests/scenarios/*.json` in one process, in name order, with the
//! same artifacts; it prints one line per scenario and the full report of each one that fails.

use std::fs;
use std::path::Path;
use std::time::Instant;

use openpaint_devtools::{Checkpoint, Export, Record, Scenario, diff, is_file_name};
use openpaint_runtime::TextFonts;

use crate::error::HarnessError;
use crate::launch::{fresh_library, text_fonts};
use crate::output::{read_rgba, workspace_root, write_rgba};

/// What one scenario run reports, line by line.
type Log = Vec<String>;

pub fn run(name: &str) -> Result<(), HarnessError> {
    let mut log = Log::new();
    let result = replay(name, &text_fonts(), &mut log);
    for line in &log {
        println!("{line}");
    }
    result
}

/// Every scenario, one after another; fails when any of them fails.
pub fn run_all() -> Result<(), HarnessError> {
    let started = Instant::now();
    let fonts = text_fonts();
    let names = scenario_names()?;
    let mut failed = 0;
    for name in &names {
        let start = Instant::now();
        let mut log = Log::new();
        let result = replay(name, &fonts, &mut log);
        let seconds = start.elapsed().as_secs_f64();
        match result {
            Ok(()) => println!("pass {name} in {seconds:.1} s"),
            Err(error) => {
                for line in &log {
                    println!("  {line}");
                }
                println!("FAIL {name} in {seconds:.1} s: {error}");
                failed += 1;
            }
        }
    }
    println!(
        "{} of {} scenarios passed in {:.1} s",
        names.len() - failed,
        names.len(),
        started.elapsed().as_secs_f64()
    );
    if failed > 0 {
        return Err(HarnessError::Scenarios {
            failed,
            total: names.len(),
        });
    }
    Ok(())
}

/// The names of `tests/scenarios/*.json`, sorted.
fn scenario_names() -> Result<Vec<String>, HarnessError> {
    let mut names = Vec::new();
    for entry in fs::read_dir(workspace_root().join("tests").join("scenarios"))? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
            && let Some(stem) = path.file_stem()
        {
            names.push(stem.to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

fn replay(name: &str, fonts: &TextFonts, log: &mut Log) -> Result<(), HarnessError> {
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
    log.push(format!("scenario {}", file.display()));
    let scenario = Scenario::parse(&fs::read_to_string(&file)?)?;
    let dir = root.join("artifacts").join("scenarios").join(name);
    let library = fresh_library(dir.join("library"))?;
    let report = openpaint_devtools::run(&scenario, &library, fonts)?;
    log.push(format!("adapter {}", report.adapter));
    fs::create_dir_all(&dir)?;
    for record in &report.records {
        match record {
            Record::Checkpoint(checkpoint) => write_checkpoint(&dir, checkpoint, log)?,
            Record::Screenshot(shot) => {
                let path = dir.join(format!("{}.png", shot.name));
                write_rgba(&path, shot.screen.size(), shot.screen.pixels())?;
                log.push(format!("screenshot {}: {}", shot.name, path.display()));
            }
            Record::Export(export) => write_export(&dir, export, log)?,
            Record::View(view) => {
                let (zoom, rotation, pan) = view.actual;
                log.push(format!(
                    "view {}: {}, zoom {zoom:.3}, rotation {rotation:.1}°, pan [{:.1}, {:.1}] \
                     (expected {:?})",
                    view.name,
                    if view.passed() { "pass" } else { "FAIL" },
                    pan[0],
                    pan[1],
                    view.expected
                ));
            }
        }
    }
    let total = report.checkpoints().count() + report.exports().count() + report.views().count();
    let failed = report.checkpoints().filter(|c| !c.passed()).count()
        + report.exports().filter(|e| !e.passed()).count()
        + report.views().filter(|v| !v.passed()).count();
    log.push(format!(
        "scenario {name}: {} of {total} checkpoints, exports and views passed, {} frames, {:.2} s",
        total - failed,
        report.frames,
        report.elapsed.as_secs_f64()
    ));
    for unnamed in &report.unnamed {
        log.push(format!("unnamed {unnamed}"));
    }
    if failed > 0 {
        return Err(HarnessError::Checkpoints { failed, total });
    }
    if !report.unnamed.is_empty() {
        return Err(HarnessError::Unnamed(report.unnamed.len()));
    }
    Ok(())
}

fn write_export(dir: &Path, export: &Export, log: &mut Log) -> Result<(), HarnessError> {
    let path = dir.join(format!(
        "{}.export.{}",
        export.name,
        export.format.extension()
    ));
    fs::write(&path, &export.bytes)?;
    let [width, height] = export.size;
    log.push(format!(
        "export {}: {}, {} {width} x {height}, {} bytes, largest difference from the canvas {}, \
         mean {:.3}: {}",
        export.name,
        if export.passed() { "pass" } else { "FAIL" },
        export.format.name(),
        export.bytes.len(),
        export.largest_difference,
        export.mean_difference,
        path.display()
    ));
    Ok(())
}

fn write_checkpoint(
    dir: &Path,
    checkpoint: &Checkpoint,
    log: &mut Log,
) -> Result<(), HarnessError> {
    let name = &checkpoint.name;
    let canvas = &checkpoint.canvas;
    let reference = dir.join(format!("{name}.canvas.png"));
    if checkpoint.passed() {
        write_rgba(&reference, canvas.size(), canvas.pixels())?;
        for stale in ["actual", "diff"] {
            // Left by an earlier failure; absent is the normal case.
            let _ = fs::remove_file(dir.join(format!("{name}.{stale}.png")));
        }
        log.push(format!(
            "checkpoint {name}: pass, canvas {:016x}",
            checkpoint.actual()
        ));
        return Ok(());
    }
    log.push(format!(
        "checkpoint {name}: FAIL, canvas {:016x}, expected {:016x}",
        checkpoint.actual(),
        checkpoint.expected
    ));
    let actual = dir.join(format!("{name}.actual.png"));
    write_rgba(&actual, canvas.size(), canvas.pixels())?;
    log.push(format!("  actual canvas: {}", actual.display()));
    let marked = read_rgba(&reference).and_then(|(size, pixels)| diff(size, &pixels, canvas));
    let Some(marked) = marked else {
        log.push(format!(
            "  no diff: no passing canvas of this size at {}",
            reference.display()
        ));
        return Ok(());
    };
    let path = dir.join(format!("{name}.diff.png"));
    write_rgba(&path, marked.size, &marked.pixels)?;
    log.push(format!(
        "  diff against the last passing canvas, {} pixels differ (red): {}",
        marked.differing,
        path.display()
    ));
    if marked.differing == 0 {
        log.push("  the pixels are the last passing ones; the expected hash changed".to_string());
    }
    Ok(())
}
