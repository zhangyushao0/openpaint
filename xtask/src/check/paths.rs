//! `#[path]` siblings only go down (S118b): a module taken from a file beside its parent by
//! `#[path = "..."]` is a folder's part named by a prefix, which a module grown past its lines is
//! split into by ownership instead (a folder: `mod.rs` and its parts). The workspace's count of
//! `#[path]` attributes (every tracked `.rs` file, a line that starts with one) is held at
//! `HELD_PATH_SIBLINGS`: more are refused, fewer ask for the number to be lowered, so the ratchet
//! never slips back.

use std::path::Path;

use super::rules::HELD_PATH_SIBLINGS;
use super::{Finding, read};

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut count = 0;
    for path in files.iter().filter(|path| path.ends_with(".rs")) {
        match read(root, path) {
            Ok(text) => count += siblings(&text),
            Err(finding) => findings.push(finding),
        }
    }
    findings.extend(judged(count));
    findings
}

/// How many `#[path]` attributes `text` holds: lines that start with one.
fn siblings(text: &str) -> usize {
    text.lines()
        .filter(|line| line.trim_start().starts_with("#[path"))
        .count()
}

/// What a workspace of `count` `#[path]` siblings breaks, if anything.
fn judged(count: usize) -> Option<Finding> {
    let held = HELD_PATH_SIBLINGS;
    let detail = if count > held {
        format!(
            "{count} #[path] siblings, held at {held}: split a module into a folder by ownership \
             (mod.rs and its parts), never into #[path] siblings"
        )
    } else if count < held {
        format!("{count} #[path] siblings, held at {held}: lower HELD_PATH_SIBLINGS to {count}")
    } else {
        return None;
    };
    Some(Finding::new(
        "path-siblings",
        "xtask/src/check/rules.rs",
        detail,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_count_is_held_more_refused_fewer_lowered() {
        assert!(judged(HELD_PATH_SIBLINGS).is_none());
        assert!(judged(HELD_PATH_SIBLINGS + 1).is_some(), "more");
        let fewer = judged(HELD_PATH_SIBLINGS - 1).map(|finding| finding.detail);
        let lower = format!("lower HELD_PATH_SIBLINGS to {}", HELD_PATH_SIBLINGS - 1);
        assert!(
            fewer.is_some_and(|detail| detail.ends_with(&lower)),
            "fewer"
        );
    }

    #[test]
    fn a_sibling_is_an_attribute_that_starts_a_line() {
        let text = "mod a;\n    #[path = \"a_tests.rs\"]\nmod tests;\n// a #[path] in words\n";
        assert_eq!(siblings(text), 1);
    }
}
