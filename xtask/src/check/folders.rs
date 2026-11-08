//! Folders stay small enough to read (S110b): a folder holds at most `MAX_FOLDER_FILES` `.rs`
//! files; one already past it when the rule came is held at its count (`HELD_FOLDERS`), which may
//! only go down: more files are refused, fewer ask for the held number to be lowered, so the
//! ratchet never slips back.

use std::collections::BTreeMap;

use super::Finding;
use super::rules::{HELD_FOLDERS, MAX_FOLDER_FILES};

pub fn check(files: &[String]) -> Vec<Finding> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for path in files.iter().filter(|path| path.ends_with(".rs")) {
        let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
        *counts.entry(folder).or_default() += 1;
    }
    let mut findings = Vec::new();
    for (folder, count) in &counts {
        if let Some(finding) = judged(folder, *count) {
            findings.push(finding);
        }
    }
    for (folder, _) in HELD_FOLDERS {
        if !counts.contains_key(folder) {
            let detail = "held in rules.rs but holds no .rs file: remove its line";
            findings.push(Finding::new("folders", folder, detail));
        }
    }
    findings
}

/// What `folder` holding `count` `.rs` files breaks, if anything.
fn judged(folder: &str, count: usize) -> Option<Finding> {
    let held = HELD_FOLDERS.iter().find(|(held, _)| *held == folder);
    let detail = match held {
        Some((_, held)) if count > *held => {
            format!("{count} .rs files, held at {held}: put them in folders by responsibility")
        }
        Some((_, held)) if count < *held && count <= MAX_FOLDER_FILES => format!(
            "{count} .rs files, within {MAX_FOLDER_FILES}: remove its line from HELD_FOLDERS"
        ),
        Some((_, held)) if count < *held => {
            format!("{count} .rs files, held at {held}: lower HELD_FOLDERS to {count}")
        }
        Some(_) => return None,
        None if count > MAX_FOLDER_FILES => format!(
            "{count} .rs files, more than {MAX_FOLDER_FILES}: put them in folders by \
             responsibility"
        ),
        None => return None,
    };
    Some(Finding::new("folders", folder, detail))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_past_the_cap_is_refused_and_a_held_one_may_only_go_down() {
        assert!(judged("crates/app/src", MAX_FOLDER_FILES).is_none());
        assert!(judged("crates/app/src", MAX_FOLDER_FILES + 1).is_some());
        let (folder, held) = HELD_FOLDERS[0];
        assert!(judged(folder, held).is_none());
        assert!(judged(folder, held + 1).is_some(), "more");
        assert!(
            judged(folder, held - 1).is_some(),
            "fewer: lower the number"
        );
        assert!(
            judged(folder, MAX_FOLDER_FILES).is_some(),
            "within: drop the line"
        );
    }
}
