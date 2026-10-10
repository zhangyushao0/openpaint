//! Development harness: offscreen rendering with readback, recorded-input replay through the
//! full frame loop, and JSON scenarios with pixel checkpoints. Writes PNGs and hashes under
//! `artifacts/` (untracked). Never shipped; no window, no mouse input (DECISIONS D12).
#![forbid(unsafe_code)]

mod brush_sheet;
mod error;
mod export_timing;
mod launch;
mod output;
mod paint_timing;
mod quantiles;
mod replay;
mod scenario;
mod shell;
mod smoke;
mod stroke;
mod stroke_stats;
mod tilt_stats;

use std::process::ExitCode;

const USAGE: &str = "usage: openpaint-harness <command>\n\ncommands:\n  render-smoke   clear an offscreen GPU target, write artifacts/harness/smoke.png, print its hash\n  render-stroke  evaluate a fixed stroke, write artifacts/harness/stroke.png, print its hash\n  shell-smoke    draw one runtime frame at tablet size, write artifacts/harness/shell.png\n  replay         replay a 10,000-sample pen stroke and a finger swipe through the frame loop,\n                 report frame times, write artifacts/harness/replay.png, check its golden hash\n                 (time it with --release)\n  export-timing  export a busy 2732 x 2048 artwork at 1x, 2x and 4x as PNG in bands, report\n                 planning, GPU and encoding times (time it with --release)\n  paint-timing   draw a stroke a sample a frame over 0 to 600 strokes of each texture brush\n                 and the Ink Pen, report CPU, GPU, tiles and op entries per frame (--release)\n  stroke-stats DIR  every stroke of the library in DIR (a copy: it opens as the app does):\n                 time between samples, spacing, speed, synthesized samples, turns and the\n                 straight segments' distance from the curve by speed, gaps, the pen's tilt\n  scenario NAME  run tests/scenarios/NAME.json, check its canvas checkpoints, write PNGs and\n                 failure diffs under artifacts/scenarios/NAME/ (also `cargo scenario NAME`)\n  scenario --all every tests/scenarios/*.json in one process, one line each, the report of\n                 each failure (also `cargo scenarios`)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["render-smoke"] => smoke::run(),
        ["render-stroke"] => stroke::run(),
        ["brush-sheet"] => brush_sheet::run(),
        ["shell-smoke"] => shell::run(),
        ["replay"] => replay::run(),
        ["export-timing"] => export_timing::run(),
        ["paint-timing"] => paint_timing::run(),
        ["stroke-stats", dir] => stroke_stats::run(dir),
        ["scenario", "--all"] => scenario::run_all(),
        ["scenario", name] => scenario::run(name),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
