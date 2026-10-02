//! Source purity: lower layers never mention GPU, UI, platform or IO; product crates carry no
//! `cfg(feature)`; every crate root forbids unsafe code.

use std::path::Path;

use super::rules::{FORBIDDEN_TOKENS, RENDER_DIR, RENDER_EGUI_DIR, UNSAFE_EXEMPT};
use super::{Finding, read};

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in files.iter().filter(|path| path.ends_with(".rs")) {
        match read(root, path) {
            Ok(text) => check_source(path, &text, &mut findings),
            Err(finding) => findings.push(finding),
        }
    }
    findings
}

fn check_source(path: &str, text: &str, findings: &mut Vec<Finding>) {
    for (dir, tokens) in FORBIDDEN_TOKENS {
        if path.starts_with(dir) {
            for token in *tokens {
                report_token(path, text, token, findings);
            }
        }
    }
    if path.starts_with(RENDER_DIR) && !path.starts_with(RENDER_EGUI_DIR) {
        report_token(path, text, "egui", findings);
    }
    if path.starts_with("crates/") && text.contains("cfg(feature") {
        findings.push(Finding::new(
            "no-features",
            path,
            "cfg(feature) in a product crate",
        ));
    }
    let exempt = UNSAFE_EXEMPT.iter().any(|dir| path.starts_with(dir));
    if is_crate_root(path) && !exempt && !text.contains("#![forbid(unsafe_code)]") {
        findings.push(Finding::new(
            "unsafe",
            path,
            "crate root must carry #![forbid(unsafe_code)]",
        ));
    }
}

fn is_crate_root(path: &str) -> bool {
    path.ends_with("/src/lib.rs") || path.ends_with("/src/main.rs")
}

fn report_token(path: &str, text: &str, token: &str, findings: &mut Vec<Finding>) {
    if let Some(index) = text.lines().position(|line| line.contains(token)) {
        findings.push(Finding::new(
            "purity",
            path,
            format!("line {}: `{token}` is not allowed in this crate", index + 1),
        ));
    }
}
