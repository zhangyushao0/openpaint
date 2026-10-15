//! Development harness: offscreen rendering with readback, recorded-input replay through the
//! full frame loop, and JSON scenarios with pixel checkpoints. Writes PNGs and hashes under
//! `artifacts/` (untracked). Never shipped; no window, no mouse input (DECISIONS D12).
//! Unsafe code is denied, as everywhere; only the counting allocator (`heap.rs`, S51) expects it.
#![deny(unsafe_code)]

mod brush_sheet;
mod error;
mod export_timing;
mod features;
mod heap;
mod launch;
mod long_session;
mod long_session_store;
mod nav_timing;
mod output;
mod paint_timing;
mod predict;
mod quantiles;
mod replay;
mod scenario;
mod shell;
mod smoke;
mod stroke;
mod stroke_stats;
mod test_art;
mod test_art_kinds;
mod tilt_stats;

use std::process::ExitCode;

const USAGE: &str = "usage: openpaint-harness <command>\n\ncommands:\n  render-smoke   clear an offscreen GPU target, write artifacts/harness/smoke.png, print its hash\n  render-stroke  evaluate a fixed stroke, write artifacts/harness/stroke.png, print its hash\n  shell-smoke    draw one runtime frame at tablet size, write artifacts/harness/shell.png\n  replay         replay a 10,000-sample pen stroke and a finger swipe through the frame loop,\n                 report frame times, write artifacts/harness/replay.png, check its golden hash\n                 (time it with --release)\n  export-timing  export a busy 2732 x 2048 artwork at 1x, 2x and 4x as PNG in bands, report\n                 planning, GPU and encoding times (time it with --release)\n  paint-timing   draw a stroke a sample a frame over 0 to 600 strokes of each texture brush\n                 and the Ink Pen, report CPU, GPU, tiles and op entries per frame (--release)\n  long-session   3,000 strokes of the built-in brushes over 60 simulated minutes with smudges,\n                 undo, layer pictures and kept versions; memory, arena, caches, history,\n                 library size and versions every 5 minutes against bounds (--release)\n  stroke-stats DIR  every stroke of the library in DIR (a copy: it opens as the app does):\n                 time between samples, spacing, speed, synthesized samples, turns and the\n                 straight segments' distance from the curve by speed, gaps, the pen's tilt\n  nav-timing DIR the last artwork of the library in DIR (a copy) opened, pinched to 2x and 4x\n                 over its busiest place, panned 10 s, drawn on while a tier fills; per phase\n                 frame CPU and GPU times and render's counts (--release)\n  bench [SCENE]  the evaluator's micro-benchmark: each op kind's GPU time per pixel over 256\n                 tiles at 1x and 4x, as `cargo device-input bench` runs it on the tablet\n  test-art KIND [N] [OUT]  a test artwork as an OpenPaint file: pile, a4 or session\n                 (also `cargo test-art`), deterministic\n  features DIR   per artwork of the library in DIR (a copy): what it uses, counts only\n  predict FILE   the tablet's GPU time for the device benchmarks' evaluations of an\n                 OpenPaint file, from the cost table, beside this GPU's (--release)\n  scenario NAME  run tests/scenarios/NAME.json, check its canvas checkpoints, write PNGs and\n                 failure diffs under artifacts/scenarios/NAME/ (also `cargo scenario NAME`)\n  scenario --all every tests/scenarios/*.json in one process, one line each, the report of\n                 each failure (also `cargo scenarios`)";

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
        ["long-session"] => long_session::run(long_session::STROKES),
        ["long-session", strokes] => match strokes.parse() {
            Ok(strokes) => long_session::run(strokes),
            Err(_) => Err(error::HarnessError::Missing(
                "long-session takes a stroke count",
            )),
        },
        ["stroke-stats", dir] => stroke_stats::run(dir),
        ["features", dir] => features::run(dir),
        ["predict", file] => predict::run(file, None),
        ["predict", file, sizes] => predict::run(file, Some(sizes)),
        ["nav-timing", dir] => nav_timing::run(dir),
        ["bench"] => bench(None),
        ["bench", scene] => bench(Some(scene)),
        ["test-art", ref rest @ ..] => test_art::run(rest),
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

/// The evaluator's micro-benchmark (S55) on this machine's GPU, as the tablet runs it.
fn bench(only: Option<&str>) -> Result<(), error::HarnessError> {
    let core = openpaint_render::Core::headless()?;
    for line in openpaint_runtime::bench(&core, only)? {
        println!("{line}");
    }
    Ok(())
}
