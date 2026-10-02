//! Development harness: offscreen rendering with readback, recorded-input replay through the
//! full frame loop, and JSON scenarios with pixel checkpoints. Writes PNGs and hashes under
//! `artifacts/` (untracked). Never shipped; no window, no mouse input (DECISIONS D12).
#![forbid(unsafe_code)]

mod error;
mod output;
mod shell;
mod smoke;

use std::process::ExitCode;

const USAGE: &str = "usage: openpaint-harness <command>\n\ncommands:\n  render-smoke  clear an offscreen GPU target, write artifacts/harness/smoke.png, print its hash\n  shell-smoke   draw one runtime frame at tablet size, write artifacts/harness/shell.png";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["render-smoke"] => smoke::run(),
        ["shell-smoke"] => shell::run(),
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
