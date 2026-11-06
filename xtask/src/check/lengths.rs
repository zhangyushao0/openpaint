//! File-length rules: Rust files and the fixed set of markdown documents (lines, characters and,
//! for the four documents, the longest line).

use std::path::Path;

use super::rules::{ARCHIVE_DIR, DOCS, Doc, MAX_DOC_LINE_CHARS, MAX_RUST_LINES};
use super::{Finding, read};

pub fn check(root: &Path, files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in files {
        if path.ends_with(".rs") {
            check_rust(root, path, &mut findings);
        } else if path.ends_with(".md") {
            check_markdown(root, path, &mut findings);
        }
    }
    findings
}

fn check_rust(root: &Path, path: &str, findings: &mut Vec<Finding>) {
    match read(root, path).map(|text| text.lines().count()) {
        Ok(lines) if lines > MAX_RUST_LINES => findings.push(Finding::new(
            "file-length",
            path,
            format!("{lines} lines, limit {MAX_RUST_LINES}; split by responsibility"),
        )),
        Ok(_) => {}
        Err(finding) => findings.push(finding),
    }
}

fn check_markdown(root: &Path, path: &str, findings: &mut Vec<Finding>) {
    if path.starts_with(ARCHIVE_DIR) {
        return;
    }
    let Some(doc) = DOCS.iter().find(|doc| doc.path == path) else {
        findings.push(Finding::new(
            "doc-set",
            path,
            "markdown outside the allowed set; fold it into one of the four docs or docs/archive/",
        ));
        return;
    };
    match read(root, path) {
        Ok(text) => findings.extend(doc_findings(doc, &text)),
        Err(finding) => findings.push(finding),
    }
}

/// The limits one allowed document passes: its lines, its characters and any line too long.
fn doc_findings(doc: &Doc, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines = text.lines().count();
    if lines > doc.lines {
        let detail = format!(
            "{lines} lines, limit {}; delete stale content or archive it",
            doc.lines
        );
        findings.push(Finding::new("doc-length", doc.path, detail));
    }
    let Some(limit) = doc.chars else {
        return findings;
    };
    let chars = text.chars().count();
    if chars > limit {
        let detail =
            format!("{chars} characters, limit {limit}; delete stale content or archive it");
        findings.push(Finding::new("doc-chars", doc.path, detail));
    }
    for (index, line) in text.lines().enumerate() {
        let chars = line.chars().count();
        if chars > MAX_DOC_LINE_CHARS {
            let number = index + 1;
            let detail = format!(
                "line {number}: {chars} characters, limit {MAX_DOC_LINE_CHARS}; one fact a line, no history"
            );
            findings.push(Finding::new("doc-line", doc.path, detail));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(doc: &Doc, text: &str) -> Vec<String> {
        doc_findings(doc, text)
            .into_iter()
            .map(|finding| format!("[{}] {}: {}", finding.rule, finding.path, finding.detail))
            .collect()
    }

    #[test]
    fn a_document_past_its_characters_is_refused_naming_file_and_limit() {
        let doc = Doc {
            path: "docs/STATUS.md",
            lines: 150,
            chars: Some(40),
        };
        assert!(found(&doc, "short\nlines\n").is_empty());
        let text = format!("{}\n{}\n", "a".repeat(20), "b".repeat(20));
        assert_eq!(
            found(&doc, &text),
            [
                "[doc-chars] docs/STATUS.md: 42 characters, limit 40; delete stale content or archive it"
            ]
        );
    }

    #[test]
    fn a_line_past_the_longest_is_refused_naming_file_line_and_limit() {
        let doc = Doc {
            path: "docs/ROADMAP.md",
            lines: 250,
            chars: Some(40_000),
        };
        // Characters, not bytes: a line of Chinese at the limit holds three times its bytes.
        let at_limit = "大".repeat(MAX_DOC_LINE_CHARS);
        let past = "x".repeat(MAX_DOC_LINE_CHARS + 1);
        let text = format!("# Roadmap\n{at_limit}\n{past}\n");
        assert_eq!(
            found(&doc, &text),
            [
                "[doc-line] docs/ROADMAP.md: line 3: 1501 characters, limit 1500; one fact a line, no history"
            ]
        );
        let agents = Doc {
            path: "AGENTS.md",
            lines: 150,
            chars: None,
        };
        assert!(found(&agents, &text).is_empty(), "only the four documents");
    }

    #[test]
    fn a_document_past_its_lines_is_refused() {
        let doc = Doc {
            path: "README.md",
            lines: 2,
            chars: None,
        };
        assert_eq!(
            found(&doc, "a\nb\nc\n"),
            ["[doc-length] README.md: 3 lines, limit 2; delete stale content or archive it"]
        );
    }
}
