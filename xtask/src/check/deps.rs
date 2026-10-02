//! Layering rules read from each crate's Cargo.toml: dependencies only point down, and product
//! crates carry no cargo features.

use std::path::Path;

use super::rules::LAYERS;
use super::{Finding, read};

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in files.iter().filter(|path| is_crate_manifest(path)) {
        let dir = path.trim_end_matches("/Cargo.toml");
        let Some((_, allowed)) = LAYERS.iter().find(|(layer, _)| *layer == dir) else {
            findings.push(Finding::new(
                "layering",
                path,
                "crate missing from LAYERS in xtask/src/check/rules.rs; add it with its allowed dependencies",
            ));
            continue;
        };
        match read(root, path) {
            Ok(text) => check_manifest(path, dir, &text, allowed, &mut findings),
            Err(finding) => findings.push(finding),
        }
    }
    findings
}

fn is_crate_manifest(path: &str) -> bool {
    path.ends_with("/Cargo.toml") && (path.starts_with("crates/") || path.starts_with("apps/"))
}

fn check_manifest(
    path: &str,
    dir: &str,
    text: &str,
    allowed: &[&str],
    findings: &mut Vec<Finding>,
) {
    for name in dependency_names(text) {
        if name.starts_with("openpaint-") && !allowed.contains(&name.as_str()) {
            findings.push(Finding::new(
                "layering",
                path,
                format!("depends on {name}, which is not below this crate; allowed: {allowed:?}"),
            ));
        }
    }
    if dir.starts_with("crates/") && text.lines().any(|line| line.trim() == "[features]") {
        findings.push(Finding::new(
            "no-features",
            path,
            "product crates have no cargo features; diagnostics go in a separate crate",
        ));
    }
}

/// Names from every `[...dependencies]` table of a manifest.
fn dependency_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_dependencies = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_dependencies = line.ends_with("dependencies]");
        } else if in_dependencies && !line.is_empty() && !line.starts_with('#') {
            let name = line.split(['=', '.']).next().unwrap_or("").trim();
            names.push(name.to_string());
        }
    }
    names
}
