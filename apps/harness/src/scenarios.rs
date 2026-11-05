//! `scenario --all [--jobs N]`: every `tests/scenarios/*.json` replayed on `N` workers at once
//! (S103), each scenario by a child process of this harness running `scenario <name>`, exactly as
//! a lone `cargo scenario <name>` runs it: the same library, artifacts, report and hashes. Child
//! processes, not threads: each scenario opens its own GPU device, and devices opened at once in
//! one process deadlocked NVIDIA's Windows driver (STATUS). It prints one line per scenario in
//! name order, each as soon as those before it are printed, the full report of each that fails,
//! and fails when any of them fails.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

use crate::error::HarnessError;
use crate::output::workspace_root;

/// How one scenario's child process went, and how long it took.
struct Outcome {
    seconds: f64,
    run: std::io::Result<Output>,
}

/// Every scenario on `jobs` workers (`default_jobs` when not given).
pub fn run_all(jobs: Option<usize>) -> Result<(), HarnessError> {
    let started = Instant::now();
    let names = scenario_names()?;
    let jobs = jobs
        .unwrap_or_else(default_jobs)
        .clamp(1, names.len().max(1));
    let harness = std::env::current_exe()?;
    let next = AtomicUsize::new(0);
    let (send, receive) = mpsc::channel();
    let failed = thread::scope(|scope| {
        let (names, harness, next) = (&names, &harness, &next);
        for _ in 0..jobs {
            let send = send.clone();
            // Each worker takes the next scenario no other took, until none is left.
            scope.spawn(move || {
                while let Some(name) = names.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if send.send(replay(harness, name)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(send);
        print_in_order(names, &receive)
    });
    println!(
        "{} of {} scenarios passed in {:.1} s on {jobs} workers",
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

/// Workers when `--jobs` is not given: the machine's threads. On the dev machine's 12 the 72
/// scenarios took 193 s on 6 workers, 172 s on 8 and 166 s on 12 (S103; 887 s one after another).
fn default_jobs() -> usize {
    thread::available_parallelism().map_or(1, usize::from)
}

/// `scenario <name>` in a child process of `harness`, its output kept.
fn replay(harness: &Path, name: &str) -> (String, Outcome) {
    let start = Instant::now();
    let run = Command::new(harness).args(["scenario", name]).output();
    let seconds = start.elapsed().as_secs_f64();
    (name.to_owned(), Outcome { seconds, run })
}

/// The outcomes as they come, each printed once every scenario before it in `names` is; how
/// many failed.
fn print_in_order(names: &[String], outcomes: &Receiver<(String, Outcome)>) -> usize {
    let mut waiting: Vec<Option<Outcome>> = names.iter().map(|_| None).collect();
    let (mut printed, mut failed) = (0, 0);
    for (name, outcome) in outcomes {
        if let Some(index) = names.iter().position(|n| *n == name) {
            waiting[index] = Some(outcome);
        }
        while let Some(outcome) = waiting.get_mut(printed).and_then(Option::take) {
            if !report(&names[printed], &outcome) {
                failed += 1;
            }
            printed += 1;
        }
    }
    failed
}

/// One line for `name`, after the child's report, indented, when it failed; whether it passed.
fn report(name: &str, outcome: &Outcome) -> bool {
    let seconds = outcome.seconds;
    let output = match &outcome.run {
        Ok(output) => output,
        Err(error) => {
            println!("FAIL {name} in {seconds:.1} s: the harness did not start: {error}");
            return false;
        }
    };
    let passed = output.status.success();
    if passed {
        println!("pass {name} in {seconds:.1} s");
    } else {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            println!("  {line}");
        }
    }
    // `main` ends a failed command with "error: <why>"; any other line (a warning, a panic's
    // message) is shown as it came.
    let mut why = None;
    for line in String::from_utf8_lossy(&output.stderr).lines() {
        match line.strip_prefix("error: ") {
            Some(error) if !passed => why = Some(error.to_owned()),
            _ => println!("  {line}"),
        }
    }
    if !passed {
        let why = why.unwrap_or_else(|| format!("the harness ended with {}", output.status));
        println!("FAIL {name} in {seconds:.1} s: {why}");
    }
    passed
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
