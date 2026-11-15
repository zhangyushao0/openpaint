//! `scenario --all [--jobs N] [--keep | --compare]`: every `tests/scenarios/*.json` replayed on
//! `N` workers at once (S103), each scenario by a child process of this harness running
//! `scenario <name>`, exactly as a lone `cargo scenario <name>` runs it: the same library,
//! artifacts, report and hashes. Child processes, not threads: each scenario opens its own GPU
//! device, and devices opened at once in one process deadlocked NVIDIA's Windows driver (STATUS).
//! It prints one line per scenario in name order, each as soon as those before it are printed,
//! the full report of each that fails, and fails when any of them fails. A scenario whose
//! process died of memory or the GPU under the load (`lost`) is replayed alone once at the end,
//! and the run says so (S129). `--keep` then keeps the run as the baseline, `--compare` compares
//! it with the baseline (`baseline.rs`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

use crate::baseline;
use crate::error::HarnessError;
use crate::output::workspace_root;

/// What `scenario --all` does after the run.
#[derive(Debug, PartialEq, Eq)]
pub enum Then {
    Nothing,
    /// Keep the run as the baseline.
    Keep,
    /// Compare the run with the baseline.
    Compare,
}

/// `--jobs N`, `--keep` and `--compare` after `scenario --all`, in any order; `None` when one
/// is unknown, repeated or malformed.
pub fn options(args: &[&str]) -> Option<(Option<usize>, Then)> {
    let (mut jobs, mut then) = (None, Then::Nothing);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match *arg {
            "--jobs" if jobs.is_none() => {
                jobs = Some(args.next()?.parse().ok().filter(|jobs| *jobs >= 1)?);
            }
            "--keep" if then == Then::Nothing => then = Then::Keep,
            "--compare" if then == Then::Nothing => then = Then::Compare,
            _ => return None,
        }
    }
    Some((jobs, then))
}

/// How one scenario's child process went, and how long it took.
struct Outcome {
    seconds: f64,
    run: std::io::Result<Output>,
}

/// Every scenario on `jobs` workers (`default_jobs` when not given), then what `then` says.
pub fn run_all(jobs: Option<usize>, then: &Then) -> Result<(), HarnessError> {
    let started = Instant::now();
    let tree = baseline::tree();
    let names = scenario_names()?;
    let jobs = jobs
        .unwrap_or_else(default_jobs)
        .clamp(1, names.len().max(1));
    let harness = std::env::current_exe()?;
    let next = AtomicUsize::new(0);
    let (send, receive) = mpsc::channel();
    let (mut failed, lost) = thread::scope(|scope| {
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
    for name in &lost {
        if !replay_alone(&harness, name, "its process was lost under the load") {
            failed += 1;
        }
    }
    println!(
        "{} of {} scenarios passed in {:.1} s on {jobs} workers{}",
        names.len() - failed,
        names.len(),
        started.elapsed().as_secs_f64(),
        if lost.is_empty() {
            String::new()
        } else {
            format!("; replayed alone after losing them: {}", lost.join(", "))
        }
    );
    let after = match then {
        Then::Nothing => Ok(()),
        Then::Keep => baseline::keep(&names, &tree),
        Then::Compare => baseline::compare(&names, &tree, &harness),
    };
    if failed > 0 {
        return Err(HarnessError::Scenarios {
            failed,
            total: names.len(),
        });
    }
    after
}

/// Workers when `--jobs` is not given: the machine's threads. On the dev machine's 12 the 72
/// scenarios took 193 s on 6 workers, 172 s on 8 and 166 s on 12 (S103; 887 s one after another).
fn default_jobs() -> usize {
    thread::available_parallelism().map_or(1, usize::from)
}

/// Where the scenarios write what they saw, a directory each.
pub fn run_dir() -> PathBuf {
    workspace_root().join("artifacts").join("scenarios")
}

/// `scenario <name>` in a child process of `harness`, its output kept.
fn replay(harness: &Path, name: &str) -> (String, Outcome) {
    let start = Instant::now();
    let run = Command::new(harness).args(["scenario", name]).output();
    let seconds = start.elapsed().as_secs_f64();
    (name.to_owned(), Outcome { seconds, run })
}

/// `name` replayed alone, `why` said first, its line printed; whether it passed.
pub fn replay_alone(harness: &Path, name: &str, why: &str) -> bool {
    println!("replaying {name} alone: {why}");
    report(name, &replay(harness, name).1)
}

/// The outcomes as they come, each printed once every scenario before it in `names` is; how
/// many failed, and the scenarios lost under the load (`lost`), which count as neither.
fn print_in_order(
    names: &[String],
    outcomes: &Receiver<(String, Outcome)>,
) -> (usize, Vec<String>) {
    let mut waiting: Vec<Option<Outcome>> = names.iter().map(|_| None).collect();
    let (mut printed, mut failed, mut lost) = (0, 0, Vec::new());
    for (name, outcome) in outcomes {
        if let Some(index) = names.iter().position(|n| *n == name) {
            waiting[index] = Some(outcome);
        }
        while let Some(outcome) = waiting.get_mut(printed).and_then(Option::take) {
            let name = &names[printed];
            if let Some(why) = outcome.run.as_ref().ok().and_then(lost_to) {
                println!(
                    "lost {name} in {:.1} s: {why}; replayed alone at the end",
                    outcome.seconds
                );
                lost.push(name.clone());
            } else if !report(name, &outcome) {
                failed += 1;
            }
            printed += 1;
        }
    }
    (failed, lost)
}

/// Why a scenario's process was lost to memory or the GPU under the load, not to what it
/// checks: Windows' fail-fast (0xc0000409, Rust's abort when an allocation fails) or access
/// violation (0xc0000005, a driver short of memory), a GPU error from the device or its memory,
/// or the driver's reset of the device that the runner refuses (the app made the display again
/// and says so on its screens, `devtools/src/runner.rs`). `None` for any other end.
fn lost_to(output: &Output) -> Option<String> {
    if output.status.success() {
        return None;
    }
    let code = output.status.code().map(|code| code.cast_unsigned());
    if let Some(code @ (0xc000_0409 | 0xc000_0005)) = code {
        return Some(format!("the process ended {code:#x}"));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let gpu = [
        "gpu device error",
        "out of memory",
        "device lost",
        "device was lost",
    ];
    let line = (stderr.lines()).find(|line| {
        let line = line.to_lowercase();
        gpu.iter().any(|gpu| line.contains(gpu))
    })?;
    Some(format!("\"{}\"", line.trim()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_after_all_are_read_in_any_order_once_each() {
        assert_eq!(options(&[]), Some((None, Then::Nothing)));
        assert_eq!(
            options(&["--keep", "--jobs", "8"]),
            Some((Some(8), Then::Keep))
        );
        assert_eq!(options(&["--compare"]), Some((None, Then::Compare)));
        for wrong in [
            &["--jobs", "0"][..],
            &["--jobs"],
            &["--keep", "--compare"],
            &["--jobs", "2", "--jobs", "3"],
            &["--fast"],
        ] {
            assert_eq!(options(wrong), None, "{wrong:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_process_lost_to_memory_or_the_gpu_is_told_from_a_failure() {
        use std::os::windows::process::ExitStatusExt;
        use std::process::ExitStatus;
        let ended = |code: u32, stderr: &str| Output {
            status: ExitStatus::from_raw(code),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        };
        assert_eq!(
            lost_to(&ended(0xc000_0409, "")).as_deref(),
            Some("the process ended 0xc0000409")
        );
        assert_eq!(
            lost_to(&ended(1, "a\nerror: GPU device error: Out of memory\n")).as_deref(),
            Some("\"error: GPU device error: Out of memory\"")
        );
        assert_eq!(
            lost_to(&ended(1, "error: 1 of 3 checkpoints failed\n")),
            None
        );
        let reset = "error: device lost: the driver reset the GPU, not a step";
        assert_eq!(
            lost_to(&ended(1, reset)).as_deref(),
            Some(format!("\"{reset}\"").as_str())
        );
        assert_eq!(lost_to(&ended(101, "thread panicked\n")), None);
        assert_eq!(lost_to(&ended(0, "out of memory")), None);
    }
}
