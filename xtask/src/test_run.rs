//! The gate's tests (S103): every test binary of the workspace, built by one `cargo test
//! --workspace --no-run`, then run as many at once as the machine has threads, those that took
//! longest last time first (`target/gate-test-times.txt`), each in a process of its own from its
//! package's directory as `cargo test` runs it, while the doc tests run beside them. `cargo test`
//! runs the binaries one after another, so the gate waited for the sum of the slow GPU binaries
//! (5 of its 6 minutes). One line per binary as it ends (its result and time), the whole output
//! of each that fails; fails when any does. Since S106b a binary still running after `HANG_LIMIT`
//! is killed and fails, the tests libtest saw running past a minute named (`guard.rs`), and a
//! binary starts only while enough memory stays free (`memory.rs`).

mod guard;
mod memory;

use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::process::{cargo_command, root};
use memory::Memory;

/// How long a test binary, or the doc tests, may run before it is killed as hung: the slowest
/// binary takes about 105 s beside the others.
const HANG_LIMIT: Duration = Duration::from_secs(600);

/// A test binary cargo built.
struct Binary {
    /// Its source, from the workspace root (`crates/runtime/tests/runtime/main.rs`).
    source: String,
    executable: PathBuf,
    /// Its package's directory, where cargo runs it.
    dir: PathBuf,
}

pub fn run() -> Result<(), String> {
    let mut binaries = build()?;
    // No times kept (a first run): cargo's order.
    longest_first(
        &mut binaries,
        &std::fs::read_to_string(times_file()).unwrap_or_default(),
    );
    let memory = Memory::watch();
    let started = Instant::now();
    let jobs = thread::available_parallelism().map_or(1, usize::from);
    let docs = thread::spawn(|| {
        let mut command = cargo_command();
        command
            .args(["test", "--workspace", "--doc", "--quiet"])
            .current_dir(root());
        guard::run(&mut command, HANG_LIMIT)
    });
    let (failed, times) = run_all(&binaries, jobs, &memory);
    if let Err(error) = std::fs::write(times_file(), times) {
        println!("  the binaries' times are not kept: {error}");
    }
    let docs = docs
        .join()
        .map_err(|_| "the doc tests' thread panicked".to_owned())?;
    let docs_passed = report("the doc tests", docs);
    println!(
        "  {} of {} test binaries passed in {:.1}s on {jobs} workers, the doc tests {}",
        binaries.len() - failed,
        binaries.len(),
        started.elapsed().as_secs_f64(),
        if docs_passed { "too" } else { "failed" }
    );
    println!("  {}", memory.summary());
    if failed > 0 || !docs_passed {
        return Err(format!("{failed} test binaries failed"));
    }
    Ok(())
}

/// Every test binary, built; their list from cargo's messages (its diagnostics go to the
/// terminal as they come).
fn build() -> Result<Vec<Binary>, String> {
    let started = Instant::now();
    let output = cargo_command()
        .args([
            "test",
            "--workspace",
            "--no-run",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(root())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("cannot start cargo test --no-run: {error}"))?;
    if !output.status.success() {
        return Err("cargo test --workspace --no-run failed".to_owned());
    }
    let messages = String::from_utf8_lossy(&output.stdout);
    let binaries: Vec<Binary> = messages.lines().filter_map(binary).collect();
    println!(
        "  cargo test --workspace --no-run took {:.1}s: {} test binaries",
        started.elapsed().as_secs_f64(),
        binaries.len()
    );
    Ok(binaries)
}

/// The test binary a cargo message announces: an artifact of the test profile with an
/// executable.
fn binary(message: &str) -> Option<Binary> {
    if !message.starts_with("{\"reason\":\"compiler-artifact\"") {
        return None;
    }
    let profile = &message[message.find("\"profile\":{")?..];
    if !profile[..profile.find('}')?].contains("\"test\":true") {
        return None;
    }
    let executable = PathBuf::from(string(message, "executable")?);
    let manifest = PathBuf::from(string(message, "manifest_path")?);
    let dir = manifest.parent()?.to_path_buf();
    let source = PathBuf::from(string(message, "src_path")?);
    let source = source.strip_prefix(root()).unwrap_or(&source);
    let source = source.to_string_lossy().replace('\\', "/");
    Some(Binary {
        source,
        executable,
        dir,
    })
}

/// The string value of `"key"` in a cargo message (compact JSON), unescaped; `None` when the
/// key is missing or its value is not a string.
fn string(message: &str, key: &str) -> Option<String> {
    let start = message.find(&format!("\"{key}\":\""))? + key.len() + 4;
    let mut value = String::new();
    let mut chars = message[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(value),
            '\\' => match chars.next()? {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                escaped => value.push(escaped),
            },
            c => value.push(c),
        }
    }
    None
}

/// Where each binary's time in the last run is kept, a line each: "<seconds> <source>".
fn times_file() -> PathBuf {
    root().join("target").join("gate-test-times.txt")
}

/// The binaries that took longest in the `last` run first, those without a time before them: the
/// slow GPU binaries start at once instead of after the quick ones cargo lists first.
fn longest_first(binaries: &mut [Binary], last: &str) {
    let seconds = |binary: &Binary| {
        last.lines()
            .find_map(|line| {
                let (seconds, source) = line.split_once(' ')?;
                (source == binary.source).then(|| seconds.parse::<f64>().ok())?
            })
            .unwrap_or(f64::MAX)
    };
    binaries.sort_by(|a, b| seconds(b).total_cmp(&seconds(a)));
}

/// Every binary on `jobs` workers, each started once `memory` admits it, a line each as it
/// ends; how many failed, and each one's time ("<seconds> <source>" lines).
fn run_all(binaries: &[Binary], jobs: usize, memory: &Memory) -> (usize, String) {
    let (next, running) = (AtomicUsize::new(0), AtomicUsize::new(0));
    let (send, receive) = mpsc::channel();
    thread::scope(|scope| {
        let (next, running) = (&next, &running);
        for _ in 0..jobs.min(binaries.len()) {
            let send = send.clone();
            scope.spawn(move || {
                while let Some(binary) = binaries.get(next.fetch_add(1, Ordering::Relaxed)) {
                    memory.admit(running);
                    let started = Instant::now();
                    let mut command = Command::new(&binary.executable);
                    command
                        .current_dir(&binary.dir)
                        .env("CARGO_MANIFEST_DIR", &binary.dir);
                    let run = guard::run(&mut command, HANG_LIMIT);
                    running.fetch_sub(1, Ordering::SeqCst);
                    let seconds = started.elapsed().as_secs_f64();
                    if send.send((binary, seconds, run)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(send);
        let (mut failed, mut times) = (0, String::new());
        for (binary, seconds, run) in receive {
            let source = &binary.source;
            times.push_str(&format!("{seconds:.1} {source}\n"));
            if !report(source, run) {
                failed += 1;
            }
        }
        (failed, times)
    })
}

/// A line for how `what` ended, after its whole output when it failed; whether it passed.
fn report(what: &str, run: io::Result<guard::Run>) -> bool {
    let run = match run {
        Ok(run) => run,
        Err(error) => {
            println!("  FAIL {what}: cannot run it: {error}");
            return false;
        }
    };
    let seconds = run.elapsed.as_secs_f64();
    let stdout = String::from_utf8_lossy(&run.stdout);
    if run.passed() {
        println!("  pass {what}: {} in {seconds:.1}s", passed(&stdout));
        return true;
    }
    print!("{stdout}");
    print!("{}", String::from_utf8_lossy(&run.stderr));
    match run.status {
        Some(status) => println!("  FAIL {what} in {seconds:.1}s: {status}"),
        None => println!(
            "  FAIL {what}: killed as hung after {seconds:.1}s (limit {}s); {}",
            HANG_LIMIT.as_secs(),
            hung(&stdout)
        ),
    }
    false
}

/// "<n> passed" from libtest's summary lines, one a binary (the doc tests have one a crate);
/// "no summary" without one.
fn passed(stdout: &str) -> String {
    let counts: Vec<u64> = (stdout.lines())
        .filter_map(|line| line.strip_prefix("test result: ok. "))
        .filter_map(|summary| summary.split(' ').next()?.parse().ok())
        .collect();
    if counts.is_empty() {
        "no summary".to_owned()
    } else {
        format!("{} passed", counts.iter().sum::<u64>())
    }
}

/// libtest's lines on the tests running past a minute that never ended, quoted; libtest prints
/// one when a test passes 60 s, and "test <name> ... ok" (or FAILED) when it ends.
fn hung(stdout: &str) -> String {
    let warned = stdout.lines().filter_map(|line| {
        let name = line
            .strip_prefix("test ")?
            .strip_suffix(" has been running for over 60 seconds")?;
        let ended = format!("test {name} ... ");
        (!stdout.lines().any(|line| line.starts_with(&ended))).then_some(line)
    });
    let quoted: Vec<String> = warned.map(|line| format!("\"{line}\"")).collect();
    if quoted.is_empty() {
        "no test reported running past a minute".to_owned()
    } else {
        format!("still running: {}", quoted.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binaries_are_read_from_cargo_messages() {
        // A path under the root as cargo writes it: a backslash (Windows') escaped.
        let path = |parts: &[&str]| {
            let path = parts.iter().fold(root(), |path, part| path.join(part));
            path.display().to_string().replace('\\', "\\\\")
        };
        let manifest = path(&["crates", "app", "Cargo.toml"]);
        let source = path(&["crates", "app", "tests", "fill.rs"]);
        let executable = path(&["target", "debug", "deps", "fill-1.exe"]);
        let message = format!(
            "{{\"reason\":\"compiler-artifact\",\"package_id\":\"path+file:///x#openpaint-app@1\",\
             \"manifest_path\":\"{manifest}\",\"target\":{{\"kind\":[\"test\"],\"name\":\"fill\",\
             \"src_path\":\"{source}\",\"test\":true}},\"profile\":{{\"opt_level\":\"0\",\
             \"test\":true}},\"features\":[],\"executable\":\"{executable}\",\"fresh\":true}}"
        );
        let found = binary(&message).expect("a test binary");
        assert_eq!(found.source, "crates/app/tests/fill.rs");
        assert_eq!(found.dir, root().join("crates").join("app"));
        assert!(found.executable.ends_with("fill-1.exe"));
        let built = message.replace("\"opt_level\":\"0\",\"test\":true", "\"opt_level\":\"0\"");
        assert!(binary(&built).is_none(), "a binary of the dev profile");
        let library = message.replace("\"executable\":\"", "\"executable\":null,\"x\":\"");
        assert!(binary(&library).is_none(), "no executable");
        assert_eq!(
            string("{\"a\":\"q\\\"\\u0041\"}", "a").as_deref(),
            Some("q\"A")
        );
    }

    #[test]
    fn the_binaries_slow_last_time_go_first_and_new_ones_before_them() {
        let named = |source: &str| Binary {
            source: source.to_owned(),
            executable: PathBuf::new(),
            dir: PathBuf::new(),
        };
        let mut binaries = ["a.rs", "b.rs", "c.rs", "d.rs"].map(named);
        longest_first(&mut binaries, "0.5 a.rs\n96.3 c.rs\nx b.rs\n2.0 d.rs\n");
        let order: Vec<&str> = binaries.iter().map(|b| b.source.as_str()).collect();
        assert_eq!(order, ["b.rs", "c.rs", "d.rs", "a.rs"]);
    }

    #[test]
    fn a_hung_binary_names_the_tests_still_running_past_a_minute() {
        let stdout = "running 3 tests\n\
            test saves::slow has been running for over 60 seconds\n\
            test robustness::cut_off has been running for over 60 seconds\n\
            test saves::slow ... ok\n\
            test saves::quick ... ok\n";
        assert_eq!(
            hung(stdout),
            "still running: \"test robustness::cut_off has been running for over 60 seconds\""
        );
        assert_eq!(
            hung("running 1 test\n"),
            "no test reported running past a minute"
        );
        assert_eq!(
            passed("test result: ok. 4 passed; 0 failed\ntest result: ok. 3 passed; 0 failed\n"),
            "7 passed"
        );
        assert_eq!(passed("running 1 test\n"), "no summary");
    }
}
