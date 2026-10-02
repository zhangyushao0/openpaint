//! `cargo repo-check`: structural rules the compiler cannot enforce. Numbers and tables live
//! in `rules.rs`; each other file checks one family of rules.

mod deps;
mod hygiene;
mod lengths;
mod purity;
mod rules;

use std::path::Path;

use crate::process::{root, tracked_files};

/// One rule violation.
pub struct Finding {
    pub rule: &'static str,
    pub path: String,
    pub detail: String,
}

impl Finding {
    pub fn new(rule: &'static str, path: &str, detail: impl Into<String>) -> Self {
        Self {
            rule,
            path: path.to_string(),
            detail: detail.into(),
        }
    }
}

pub fn run() -> Result<(), String> {
    let root = root();
    let files = tracked_files()?;
    let mut findings = Vec::new();
    findings.extend(lengths::check(&root, &files));
    findings.extend(deps::check(&root, &files));
    findings.extend(purity::check(&root, &files));
    findings.extend(hygiene::check(&root, &files));
    if findings.is_empty() {
        println!("repo-check: {} tracked files, all rules hold", files.len());
        return Ok(());
    }
    for finding in &findings {
        println!("[{}] {}: {}", finding.rule, finding.path, finding.detail);
    }
    Err(format!("repo-check: {} finding(s)", findings.len()))
}

/// Reads a tracked text file; a file that cannot be read is itself a finding.
pub fn read(root: &Path, path: &str) -> Result<String, Finding> {
    std::fs::read_to_string(root.join(path))
        .map_err(|error| Finding::new("read", path, error.to_string()))
}
