//! A practice kind is told apart only in its own folder (S109, S109b, D111): outside
//! `crates/practice/` and the tests, no source that names the practice crate matches over its
//! `Kind` (an arm or a `matches!` naming `Kind::`), names a `Family` or an `Original` variant at
//! all (an arm, a `matches!`, an `if let`, a comparison: what a family or an original does is its
//! kind's description's, `Facts` and the description's methods), or names the retired `Overlay`.
//! A kind is built outside only where a stored code is read (`Kind::Scale` in the practice
//! records' upgrade of 1.14.0's scales, `crates/practice-store`). The flow (`crates/practice-app`)
//! asks the facts as the app did. Core, raster, render, app and the store name no practice at all
//! (`rules.rs`, S110, S110b); the UI follows with S110c.

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
            // The practice crate itself, not `openpaint_practice_store`.
            Ok(text) if text.contains("openpaint_practice::") => {
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
        let arm = names(code, "Kind::") && (code.contains("=>") || code.contains("matches!("));
        let told = names_variant(code, "Family::") || names_variant(code, "Original::");
        if arm || told || code.contains("Overlay") {
            let detail = format!(
                "line {}: a practice kind, family or original told apart; ask its description \
                 (Kind::facts, Family::facts, Exercise::compare)",
                index + 1
            );
            findings.push(Finding::new("kinds", path, detail));
        }
    }
    findings
}

/// Where `code` names `path` (`Kind::`, not `ErrorKind::` or another name ending so): the byte
/// offsets just after it.
fn named<'a>(code: &'a str, path: &'a str) -> impl Iterator<Item = usize> + 'a {
    code.match_indices(path).filter_map(move |(at, _)| {
        let before = code[..at].chars().next_back();
        let alone = !before.is_some_and(|c| c.is_alphanumeric() || c == '_');
        alone.then_some(at + path.len())
    })
}

/// Whether `code` names `path`.
fn names(code: &str, path: &str) -> bool {
    named(code, path).next().is_some()
}

/// Whether `code` names a variant of `path` (`Family::Line`; not `Family::ALL` or a function such
/// as `Family::of_key`): a capital, then a small letter.
fn names_variant(code: &str, path: &str) -> bool {
    named(code, path).any(|after| {
        let mut rest = code[after..].chars();
        rest.next().is_some_and(char::is_uppercase) && rest.next().is_some_and(char::is_lowercase)
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

    #[test]
    fn a_family_or_an_original_told_apart_is_refused_its_facts_are_not() {
        let told = [
            "if family == Some(Family::WholeCopy) {",
            "Some(Family::Division) => level,",
            "let Original::Scale(scale) = &original else {",
            "matches!(original, Original::Picture(_))",
        ];
        for line in told {
            assert_eq!(matches_in("crates/app/src/a.rs", line).len(), 1, "{line}");
        }
        let asked = "for family in Family::ALL {\nlet f = Family::of_key(key);\n\
                     let tools = family.facts().tools;\nlet lines = original.lines();";
        assert!(matches_in("crates/ui/src/b.rs", asked).is_empty());
        let other = "let k = SubFamily::Thing;";
        assert!(matches_in("crates/ui/src/c.rs", other).is_empty());
    }
}
