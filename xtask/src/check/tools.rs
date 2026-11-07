//! Tool workspaces outside the root one (`TOOLS`, S111: the practice-pack builder): held to the
//! repository's code rules though the gate neither builds nor tests them. Their sources' lengths,
//! scripts and layering are the other checks' (they read every tracked file); here their
//! manifest's lints are the root's word for word, and rustfmt and clippy (the function length
//! among its lints) pass on them, checked only, nothing rewritten.

use std::path::Path;

use super::rules::TOOLS;
use super::{Finding, read};
use crate::process::cargo;

pub fn check(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let root_manifest = match read(root, "Cargo.toml") {
        Ok(text) => text,
        Err(finding) => return vec![finding],
    };
    let root_lints = lint_tables(&root_manifest);
    for tool in TOOLS {
        let manifest = format!("{tool}/Cargo.toml");
        match read(root, &manifest) {
            Ok(text) if lint_tables(&text) != root_lints => findings.push(Finding::new(
                "tool-lints",
                &manifest,
                "its [workspace.lints] differ from the root manifest's; copy them word for word",
            )),
            Ok(_) => {}
            Err(finding) => {
                findings.push(finding);
                continue;
            }
        }
        let fmt = ["fmt", "--manifest-path", &manifest, "--check"];
        if let Err(error) = cargo(&fmt) {
            let detail = format!("{error}; run `cargo fmt --manifest-path {manifest}`");
            findings.push(Finding::new("tool-fmt", &manifest, detail));
        }
        let clippy = [
            "clippy",
            "--quiet",
            "--manifest-path",
            &manifest,
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ];
        if let Err(error) = cargo(&clippy) {
            findings.push(Finding::new("tool-clippy", &manifest, error));
        }
    }
    findings
}

/// The `[workspace.lints.*]` tables of a manifest: their headers and settings, comments and
/// blank lines left out.
fn lint_tables(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line.starts_with("[workspace.lints");
        }
        if inside && !line.is_empty() && !line.starts_with('#') {
            lines.push(line);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::lint_tables;

    #[test]
    fn lint_tables_are_compared_without_comments_or_other_tables() {
        let root = "[workspace]\nmembers = []\n\n[workspace.lints.rust]\nunsafe_code = \"deny\"\n\n\
                    [workspace.lints.clippy]\n# why\ntodo = \"deny\"\n";
        let tool = "[package]\nname = \"t\"\n[workspace.lints.rust]\nunsafe_code = \"deny\"\n\
                    [workspace.lints.clippy]\ntodo = \"deny\"\n[profile.dev]\ndebug = false\n";
        assert_eq!(lint_tables(root), lint_tables(tool));
        assert_ne!(lint_tables(root), lint_tables("[workspace.lints.rust]\n"));
    }
}
