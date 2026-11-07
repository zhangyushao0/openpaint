//! Development harness: offscreen rendering with readback, recorded-input replay through the
//! full frame loop, and JSON scenarios with pixel checkpoints. Writes PNGs and hashes under
//! `artifacts/` (untracked). Never shipped; no window, no mouse input (DECISIONS D12).
//! Unsafe code is denied, as everywhere; only the counting allocator (`heap.rs`) expects it.
#![deny(unsafe_code)]

mod brush_sheet;
mod error;
mod export_timing;
mod heap;
mod key_copies;
mod key_sheet;
mod key_shifts;
mod launch;
mod long_session;
mod output;
mod quantiles;
mod replay;
#[cfg(test)]
mod runner_tests;
mod scenario;
mod scenarios;
mod scratch;
mod shell;
mod smoke;
mod stroke;
mod stroke_stats;
mod test_art;
mod tilt_stats;

use std::process::ExitCode;

const USAGE: &str = "usage: openpaint-harness <command>\n\ncommands:\n  render-smoke   clear an offscreen GPU target, write artifacts/harness/smoke.png, print its hash\n  render-stroke  draw a fixed stroke, write artifacts/harness/stroke.png, print its hash\n  brush-sheet    every built-in brush's stroke at 1x and its turn at 4x, and the shape\n                 textures, as PNGs under artifacts/harness/\n  shell-smoke    draw one runtime frame at tablet size, write artifacts/harness/shell.png\n  replay         replay a 10,000-sample pen stroke and a finger swipe through the frame loop,\n                 report frame times, write artifacts/harness/replay.png, check its golden hash\n                 (time it with --release)\n  export-timing  export a busy 2732 x 2048 artwork at 1x, 2x and 4x as PNG in bands, report\n                 planning, GPU and encoding times (time it with --release)\n  long-session   3,000 strokes of the built-in brushes over 60 simulated minutes with smudges,\n                 undo, layer pictures and kept versions; memory, arena, caches, history,\n                 library size and versions every 5 minutes against bounds (--release)\n  stroke-stats DIR  every stroke of the library in DIR (opened as a copy): time between\n                 samples, spacing, speed, synthesized samples, turns and the straight\n                 segments' distance from the curve by speed, gaps, the pen's tilt\n  test-art KIND [N] [OUT]  a test artwork as an OpenPaint file: pile, a4, a4-plain, kinds or session\n                 (also `cargo test-art`), deterministic\n  scenario NAME  run tests/scenarios/NAME.json, check its canvas checkpoints, write PNGs and\n                 failure diffs under artifacts/scenarios/NAME/ (also `cargo scenario NAME`)\n  scenario --all [--jobs N]  every tests/scenarios/*.json, N at once (the machine's threads\n                 by default), each in a process of its own, one line each in name order, the\n                 report of each failure (also `cargo scenarios`)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["render-smoke"] => smoke::run(),
        ["render-stroke"] => stroke::run(),
        ["brush-sheet"] => brush_sheet::run(),
        ["shell-smoke"] => shell::run(),
        ["replay"] => replay::run(),
        ["export-timing"] => export_timing::run(),
        ["long-session"] => long_session::run(long_session::STROKES),
        ["long-session", strokes] => match strokes.parse() {
            Ok(strokes) => long_session::run(strokes),
            Err(_) => Err(error::HarnessError::Missing(
                "long-session takes a stroke count",
            )),
        },
        ["stroke-stats", dir] => stroke_stats::run(dir),
        ["test-art", ref rest @ ..] => test_art::run(rest),
        ["key-shifts", ref rest @ ..] => key_shifts::run(rest),
        ["scenario", "--all"] => scenarios::run_all(None),
        ["scenario", "--all", "--jobs", jobs] => match jobs.parse() {
            Ok(jobs) if jobs >= 1 => scenarios::run_all(Some(jobs)),
            _ => Err(error::HarnessError::Missing("--jobs takes a count from 1")),
        },
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
