//! Repository hygiene: no notes or scripts tracked, LF line endings everywhere.

use std::path::Path;

use super::Finding;
use super::rules::{FORBIDDEN_EXTENSIONS, FORBIDDEN_PREFIXES, TEXT_EXTENSIONS};

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in files {
        if let Some(prefix) = FORBIDDEN_PREFIXES
            .iter()
            .find(|prefix| path.starts_with(*prefix))
        {
            findings.push(Finding::new(
                "tracked-path",
                path,
                format!(
                    "{prefix} is never tracked; notes and artifacts stay out of the repository"
                ),
            ));
        }
        let extension = path.rsplit('.').next().unwrap_or("");
        if path.contains('.') && FORBIDDEN_EXTENSIONS.contains(&extension) {
            findings.push(Finding::new(
                "no-scripts",
                path,
                "scripts are not tracked; tooling is Rust in xtask",
            ));
        }
        if is_text(path, extension) {
            check_line_endings(root, path, &mut findings);
        }
    }
    findings
}

fn is_text(path: &str, extension: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.starts_with('.') || TEXT_EXTENSIONS.contains(&extension)
}

fn check_line_endings(root: &Path, path: &str, findings: &mut Vec<Finding>) {
    match std::fs::read(root.join(path)) {
        Ok(bytes) if bytes.windows(2).any(|pair| pair == b"\r\n") => {
            findings.push(Finding::new(
                "line-endings",
                path,
                "CRLF found; the repository is LF only (.gitattributes)",
            ));
        }
        Ok(_) => {}
        Err(error) => findings.push(Finding::new("read", path, error.to_string())),
    }
}
