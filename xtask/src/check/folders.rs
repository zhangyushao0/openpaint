//! Folders stay small enough to read (S110b): a folder holds at most `MAX_FOLDER_FILES` `.rs`
//! files. The one past it when the rule came, the renderer's root, went within it in S136.

use std::collections::BTreeMap;

use super::Finding;
use super::rules::MAX_FOLDER_FILES;

pub fn check(files: &[String]) -> Vec<Finding> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for path in files.iter().filter(|path| path.ends_with(".rs")) {
        let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
        *counts.entry(folder).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(folder, count)| judged(folder, count))
        .collect()
}

/// What `folder` holding `count` `.rs` files breaks, if anything.
fn judged(folder: &str, count: usize) -> Option<Finding> {
    (count > MAX_FOLDER_FILES).then(|| {
        let detail = format!(
            "{count} .rs files, more than {MAX_FOLDER_FILES}: put them in folders by \
             responsibility"
        );
        Finding::new("folders", folder, detail)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_past_the_cap_is_refused() {
        assert!(judged("crates/app/src", MAX_FOLDER_FILES).is_none());
        assert!(judged("crates/app/src", MAX_FOLDER_FILES + 1).is_some());
    }
}
