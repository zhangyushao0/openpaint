//! The interface's words live in its text tables (DECISIONS D100): under `crates/ui/src/` and
//! `crates/practice-ui/src/` (S110c), outside each one's `text/` and the tests, no string literal
//! reads like a word shown to the user (a capital, then a small letter: "Undo", "Can't fill"),
//! and no literal is handed to `Words::made` or `Words::named`. Ids, salts and numbers ("left
//! island", "{} %") stay where they are.

use std::path::Path;

use super::rules::UI_SOURCES;
use super::{Finding, read};

/// Whether `path` is a screen's source outside its text tables and tests.
fn screen_source(path: &str) -> bool {
    path.ends_with(".rs")
        && !path.ends_with("_tests.rs")
        && (UI_SOURCES.iter())
            .any(|(sources, tables)| path.starts_with(sources) && !path.starts_with(tables))
}

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let checked = files.iter().filter(|path| screen_source(path));
    for path in checked {
        match read(root, path) {
            Ok(text) => findings.extend(words_in(path, &text)),
            Err(finding) => findings.push(finding),
        }
    }
    findings
}

/// The findings of one source file: its lines up to its tests.
fn words_in(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("#[cfg(test)]") {
            break;
        }
        if code.starts_with("//") || code.contains("debug_struct(") || code.contains("reason = ") {
            continue;
        }
        let number = index + 1;
        if code.contains("Words::made(\"") || code.contains("Words::named(\"") {
            let detail = format!("line {number}: words made from a literal; use a text table");
            findings.push(Finding::new("words", path, detail));
        }
        for literal in literals(code).filter(|literal| reads_as_words(literal)) {
            let detail = format!("line {number}: \"{literal}\" belongs in the text tables");
            findings.push(Finding::new("words", path, detail));
        }
    }
    findings
}

/// The string literals of one line of code, without their quotes (escapes kept as written).
fn literals(code: &str) -> impl Iterator<Item = &str> {
    let mut rest = code;
    std::iter::from_fn(move || {
        let start = rest.find('"')? + 1;
        let body = &rest[start..];
        let mut escaped = false;
        let end = body.char_indices().find_map(|(at, c)| {
            let closes = c == '"' && !escaped;
            escaped = c == '\\' && !escaped;
            closes.then_some(at)
        })?;
        rest = &body[end + 1..];
        Some(&body[..end])
    })
}

/// A literal that starts like a word or sentence for the user: a capital, then a small letter.
fn reads_as_words(literal: &str) -> bool {
    let mut chars = literal.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(first), Some(second)) if first.is_ascii_uppercase() && second.is_ascii_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_for_the_user_are_found_and_ids_are_not() {
        let source = "let a = text_button(ui, \"Undo\", true);\n\
                      let b = island(ctx, \"left island\", place);\n\
                      let c = format!(\"{} %\", 5);\n\
                      // \"Comments\" are free.\n\
                      let d = Words::made(\"x\");\n\
                      f.debug_struct(\"Panel\");\n\
                      let e = (\"a \\\"Quoted\\\" b\", \"Two\");\n\
                      #[cfg(test)]\n\
                      let f = \"Tests\";";
        let found: Vec<String> = words_in("crates/ui/src/x.rs", source)
            .into_iter()
            .map(|finding| finding.detail)
            .collect();
        assert_eq!(
            found,
            [
                "line 1: \"Undo\" belongs in the text tables",
                "line 5: words made from a literal; use a text table",
                "line 7: \"Two\" belongs in the text tables",
            ]
        );
    }

    #[test]
    fn both_interfaces_are_checked_outside_their_tables_and_tests() {
        assert!(screen_source("crates/ui/src/shell.rs"));
        assert!(screen_source("crates/practice-ui/src/sheet/bar.rs"));
        assert!(!screen_source("crates/ui/src/text/canvas.rs"));
        assert!(!screen_source("crates/practice-ui/src/text/kinds/lines.rs"));
        assert!(!screen_source("crates/practice-ui/src/sheet/demo_tests.rs"));
        assert!(!screen_source("crates/runtime/src/frame/mod.rs"));
    }
}
