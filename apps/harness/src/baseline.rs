//! A run of every scenario kept as the baseline, and a later run compared with it (S129), so a
//! refactor proves "every screenshot byte for byte against the commit before" without a person:
//! `scenario --all --keep` copies each scenario's files (`artifacts/scenarios/<name>/*`, not its
//! library) to `artifacts/scenarios-baseline/` with the tree that ran them (its commit and
//! whether it was dirty) in `tree.txt`; `scenario --all --compare` compares every file of the
//! run with it (`compare.rs`), replays each scenario with a differing file alone once and
//! compares it again, prints both trees, a line a file that still differs, and fails when any
//! does.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::compare::{self, Difference};
use crate::error::HarnessError;
use crate::output::workspace_root;
use crate::scenarios::{replay_alone, run_dir};
use crate::scratch;

/// Where the baseline is kept.
fn baseline_dir() -> PathBuf {
    workspace_root()
        .join("artifacts")
        .join("scenarios-baseline")
}

/// The file in the baseline naming the tree that ran it.
const TREE: &str = "tree.txt";

/// The tree the scenarios run from: the commit and whether the working tree differs from it
/// ("d045cf7 clean", "d045cf7 dirty"); "no git" when git cannot say.
pub fn tree() -> String {
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(workspace_root())
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    match (
        git(&["rev-parse", "--short", "HEAD"]),
        git(&["status", "--porcelain"]),
    ) {
        (Some(commit), Some(changes)) => {
            format!(
                "{commit} {}",
                if changes.is_empty() { "clean" } else { "dirty" }
            )
        }
        _ => "no git".to_owned(),
    }
}

/// This run's files of `names` kept as the baseline, in place of the one kept before.
pub fn keep(names: &[String], tree: &str) -> Result<(), HarnessError> {
    let dir = baseline_dir();
    scratch::remove(&dir);
    let mut kept = 0;
    for name in names {
        let (from, to) = (run_dir().join(name), dir.join(name));
        fs::create_dir_all(&to)?;
        // A scenario that failed before writing anything has no directory.
        let Ok(entries) = fs::read_dir(&from) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_type()?.is_file() {
                fs::copy(entry.path(), to.join(entry.file_name()))?;
                kept += 1;
            }
        }
    }
    fs::write(dir.join(TREE), format!("{tree}\n"))?;
    println!(
        "kept {kept} files of {} scenarios as the baseline of {tree}: {}",
        names.len(),
        dir.display()
    );
    Ok(())
}

/// This run's files of `names` compared with the baseline, each scenario that differs replayed
/// alone once by `harness` and compared again; fails when a file still differs.
pub fn compare(names: &[String], tree: &str, harness: &Path) -> Result<(), HarnessError> {
    let dir = baseline_dir();
    let Ok(kept_tree) = fs::read_to_string(dir.join(TREE)) else {
        return Err(HarnessError::Missing(
            "no baseline kept: run `cargo scenarios --keep` on the tree to compare with first",
        ));
    };
    println!("baseline: {}; this run: {tree}", kept_tree.trim());
    // A scenario kept in the baseline but gone from the tree: its files are only in the baseline.
    let all: BTreeSet<String> = names.iter().cloned().chain(scenarios_in(&dir)).collect();
    let (mut files, mut differing) = (0, Vec::new());
    for name in &all {
        let (count, mut found) = compare::scenario(&dir, &run_dir(), name);
        files += count;
        if !found.is_empty() && names.contains(name) {
            replay_alone(harness, name, "a file differed from the baseline");
            let alone = compare::scenario(&dir, &run_dir(), name).1;
            if alone.is_empty() {
                for (file, difference) in &found {
                    println!("alike replayed alone, under the load {file}: {difference}");
                }
            }
            found = alone;
        }
        differing.extend(found);
    }
    report(&differing, files, all.len())
}

/// A line a differing file, then the verdict.
fn report(
    differing: &[(String, Difference)],
    files: usize,
    scenarios: usize,
) -> Result<(), HarnessError> {
    for (file, difference) in differing {
        println!("differs {file}: {difference}");
    }
    if differing.is_empty() {
        println!("every one of {files} files of {scenarios} scenarios is alike");
        return Ok(());
    }
    println!(
        "{} of {files} files of {scenarios} scenarios differ from the baseline",
        differing.len()
    );
    Err(HarnessError::Differ(differing.len()))
}

/// The scenarios a baseline holds: its directories.
fn scenarios_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    (entries.flatten())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}
