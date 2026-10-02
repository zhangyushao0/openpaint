//! File-length rules: Rust files and the fixed set of markdown documents.

use std::path::Path;

use super::rules::{ARCHIVE_DIR, DOCS, MAX_RUST_LINES};
use super::{Finding, read};

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in files {
        if path.ends_with(".rs") {
            check_rust(root, path, &mut findings);
        } else if path.ends_with(".md") {
            check_markdown(root, path, &mut findings);
        }
    }
    findings
}

fn line_count(root: &Path, path: &str) -> Result<usize, Finding> {
    read(root, path).map(|text| text.lines().count())
}

fn check_rust(root: &Path, path: &str, findings: &mut Vec<Finding>) {
    match line_count(root, path) {
        Ok(lines) if lines > MAX_RUST_LINES => findings.push(Finding::new(
            "file-length",
            path,
            format!("{lines} lines, limit {MAX_RUST_LINES}; split by responsibility"),
        )),
        Ok(_) => {}
        Err(finding) => findings.push(finding),
    }
}

fn check_markdown(root: &Path, path: &str, findings: &mut Vec<Finding>) {
    if path.starts_with(ARCHIVE_DIR) {
        return;
    }
    let Some((_, limit)) = DOCS.iter().find(|(allowed, _)| *allowed == path) else {
        findings.push(Finding::new(
            "doc-set",
            path,
            "markdown outside the allowed set; fold it into one of the four docs or docs/archive/",
        ));
        return;
    };
    match line_count(root, path) {
        Ok(lines) if lines > *limit => findings.push(Finding::new(
            "doc-length",
            path,
            format!("{lines} lines, limit {limit}; delete stale content or archive it"),
        )),
        Ok(_) => {}
        Err(finding) => findings.push(finding),
    }
}
