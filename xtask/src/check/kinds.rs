//! A practice kind is told apart only in its own folder (S109, S109b, D111, S123). The rule
//! checks, line by line (comments and a file's `#[cfg(test)]` part left out):
//!
//! - Everywhere (every `.rs` file of the repository): no enum declared `Family`, `Original`,
//!   `Tools` or `Picks` and no path naming a variant of one (`Family::Line`, `Picks::Swatch`).
//!   The kinds' parallel lists are gone (S123): a kind's topic is its facts' key, what it gives a
//!   value behind `Original` (a trait), its tools and picks data (`ToolSet`, `Picking`).
//! - Outside `crates/practice/src/kinds/`, in product code (not a `tests/` folder or a
//!   `_tests.rs` file) of the practice crate and of every source that names it: no match on
//!   `Kind`'s variants, that is no line naming `Kind::` with an arm (`=>`), a `matches!(`, or a
//!   `let` pattern before its `=` (`let Kind::Line(tier) = kind`, `if let`, `let (Kind::A(t) |
//!   ..)`). A kind is built outside only where a stored code is read (`Kind::of_code`) or a
//!   lesson names its kinds; the flow and the screens ask a kind's `Facts` and its original's
//!   methods. The pack builder (`tools/packs/`) is held to it too.
//! - Outside the practice crate: no `Overlay` (retired, S109).
//!
//! Core, raster, render, app and the store name no practice at all (`rules.rs`, S110, S110b).

use std::path::Path;

use super::{Finding, read};

/// Where the practice crate lives.
const PRACTICE: &str = "crates/practice/";
/// Where a kind's folder lives: its own matches are its business.
const KINDS: &str = "crates/practice/src/kinds/";
/// The kinds' retired parallel lists.
const RETIRED: [&str; 4] = ["Family", "Original", "Tools", "Picks"];

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in files.iter().filter(|path| path.ends_with(".rs")) {
        match read(root, path) {
            Ok(text) => findings.extend(findings_in(path, &text)),
            Err(finding) => findings.push(finding),
        }
    }
    findings
}

/// The findings of one source file: its lines up to its tests.
fn findings_in(path: &str, text: &str) -> Vec<Finding> {
    let practice = path.starts_with(PRACTICE);
    let product = !path.contains("/tests/") && !path.ends_with("_tests.rs");
    let practised = practice || text.contains("openpaint_practice::");
    let xtask = path.starts_with("xtask/");
    let matched = product && practised && !xtask && !path.starts_with(KINDS);
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("#[cfg(test)]") {
            break;
        }
        if code.starts_with("//") {
            continue;
        }
        let detail = if retired(code) {
            "a kinds' list retired in S123 (Family, Original, Tools, Picks as an enum); a kind's \
             topic is its facts' key, its tools and picks are data"
        } else if matched && matches_kind(code) {
            "a match on a practice kind's variants outside its folder; ask its description \
             (Kind::facts, the original's methods)"
        } else if !practice && practised && product && !xtask && code.contains("Overlay") {
            "the retired Overlay"
        } else {
            continue;
        };
        let detail = format!("line {}: {detail}", index + 1);
        findings.push(Finding::new("kinds", path, detail));
    }
    findings
}

/// Whether `code` declares one of the retired lists as an enum or names a variant of one.
fn retired(code: &str) -> bool {
    RETIRED.iter().any(|name| {
        let declared = named(code, "enum ").any(|after| word_at(code, after, name));
        declared || names_variant(code, &format!("{name}::"))
    })
}

/// Whether `code` matches on `Kind`'s variants: an arm, a `matches!`, or a `let` pattern.
fn matches_kind(code: &str) -> bool {
    let Some(first) = named(code, "Kind::").next() else {
        return false;
    };
    if code.contains("=>") || code.contains("matches!(") {
        return true;
    }
    // A `let` whose pattern (before its `=`) names a variant.
    let bound = code.find(" = ").or_else(|| code.find(" =\t"));
    let pattern = bound.is_none_or(|at| first < at);
    let lets = code.starts_with("let ") || code.contains("if let ") || code.contains("while let ");
    lets && pattern
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

/// Whether `name` stands at `at` in `code` as a word of its own (`Family`, not `Families`).
fn word_at(code: &str, at: usize, name: &str) -> bool {
    code[at..].starts_with(name)
        && !(code[at + name.len()..].chars().next())
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
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
    fn a_match_on_a_kind_outside_its_folder_is_refused_a_call_is_not() {
        let arm = "match kind {\n    Kind::Line(_) => 1,\n}";
        assert_eq!(findings_in("crates/practice/src/course/a.rs", arm).len(), 1);
        let used = "use openpaint_practice::Kind;\n";
        for matched in [
            "let Kind::Line(tier) = kind else {",
            "if let Kind::Line(tier) = kind {",
            "let (Kind::Division(t) | Kind::WholeCopy(t)) = *kind",
            "let massed = matches!(kind, Kind::ValueMasses(_));",
        ] {
            let text = format!("{used}{matched}");
            assert_eq!(
                findings_in("crates/ui/src/a.rs", &text).len(),
                1,
                "{matched}"
            );
        }
        let built = "let kind = Kind::of_code(1, 0);\nlet built = Kind::Study;\n\
                     let first = [Kind::Free, Kind::Line(LineTier::Arc)];";
        assert!(findings_in("crates/practice/src/course/b.rs", built).is_empty());
        assert!(findings_in("crates/practice/src/kinds/lines/mod.rs", arm).is_empty());
        assert!(findings_in("crates/practice/tests/kinds.rs", arm).is_empty());
        let error = "Err(e) if e.kind() == ErrorKind::NotFound => {}";
        assert!(findings_in("crates/store/src/d.rs", error).is_empty());
        let other = "match kind {\n    Kind::Png(w) => 1,\n}";
        assert!(findings_in("crates/store/src/e.rs", other).is_empty());
        let tested = "#[cfg(test)]\nmod tests { fn t() { matches!(k, Kind::Study); } }";
        assert!(findings_in("crates/practice/src/c.rs", tested).is_empty());
    }

    #[test]
    fn the_retired_lists_are_refused_as_enums_everywhere_their_names_elsewhere_are_not() {
        for line in [
            "pub enum Family {",
            "enum Picks {",
            "pub(crate) enum Tools {",
            "if family == Some(Family::WholeCopy) {",
            "let Original::Scale(scale) = &original else {",
            "Tools::Colour | Tools::Cells => None,",
            "Picks::Swatch if on_original => x,",
        ] {
            assert_eq!(
                findings_in("crates/practice/tests/a.rs", line).len(),
                1,
                "{line}"
            );
        }
        let kept = "pub trait Original: Any {\nlet on = In::Original;\nenum FontFamily {\n\
                    let f = egui::FontFamily::Proportional;\nlet tools = &facts.tools;\n\
                    let all = Family::ALL;";
        assert!(findings_in("crates/practice/src/b.rs", kept).is_empty());
    }
}
