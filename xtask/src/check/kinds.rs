//! A practice kind is matched only in its own folder (S109, D111): outside `crates/practice/` and
//! the tests, no source that names the practice crate matches over its `Kind` (an arm or a
//! `matches!` naming `Kind::`) or names the retired `Overlay`. What a kind does goes through its
//! description; the rest of S110's rule (no match over a family or an original, no painting crate
//! naming practice) comes with S110.

use std::path::Path;

use super::{Finding, read};

/// Where the practice crate lives: its own matches are its business.
const PRACTICE: &str = "crates/practice/";
/// The pack builder, a workspace of its own on the practice crate (S111).
const PACK_BUILDER: &str = "tools/packs/";

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let checked = files.iter().filter(|path| {
        path.ends_with(".rs")
            && !path.starts_with(PRACTICE)
            && !path.starts_with(PACK_BUILDER)
            && !path.starts_with("xtask/")
            && !path.contains("/tests/")
            && !path.ends_with("_tests.rs")
    });
    for path in checked {
        match read(root, path) {
            Ok(text) if text.contains("openpaint_practice") => {
                findings.extend(matches_in(path, &text));
            }
            Ok(_) => {}
            Err(finding) => findings.push(finding),
        }
    }
    findings
}

/// The findings of one source file: its lines up to its tests.
fn matches_in(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("#[cfg(test)]") {
            break;
        }
        if code.starts_with("//") {
            continue;
        }
        let arm = names_kind(code) && (code.contains("=>") || code.contains("matches!("));
        if arm || code.contains("Overlay") {
            let detail = format!(
                "line {}: a match over a practice kind; ask its description (Kind::facts)",
                index + 1
            );
            findings.push(Finding::new("kinds", path, detail));
        }
    }
    findings
}

/// Whether `code` names the practice `Kind` (not `ErrorKind` or another name ending so).
fn names_kind(code: &str) -> bool {
    code.match_indices("Kind::").any(|(at, _)| {
        let before = code[..at].chars().next_back();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arm_over_a_kind_is_refused_a_call_is_not() {
        let arm = "match kind {\n    Kind::Line(_) => 1,\n}";
        assert_eq!(matches_in("crates/ui/src/a.rs", arm).len(), 1);
        let call = "let kind = Kind::of_code(1, 0);\nlet built = Kind::Study;";
        assert!(matches_in("crates/app/src/b.rs", call).is_empty());
        let error = "Err(e) if e.kind() == ErrorKind::NotFound => {}";
        assert!(matches_in("crates/store/src/d.rs", error).is_empty());
        let tested = "#[cfg(test)]\nmod tests { fn t() { matches!(k, Kind::Study); } }";
        assert!(matches_in("crates/app/src/c.rs", tested).is_empty());
    }
}
