//! `Title`: the name an artwork carries in the gallery. The user types it; the store keeps it.

use std::fmt;

/// Most characters a title may have.
pub const MAX_TITLE_CHARS: usize = 80;

/// The title of a new artwork until the user renames it.
pub const UNTITLED: &str = "Untitled Artwork";

/// A title as the gallery shows it: trimmed, between 1 and `MAX_TITLE_CHARS` characters, no
/// control characters (no line breaks).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title(String);

/// Why a text cannot be a title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleError {
    /// Nothing but white space.
    Empty,
    TooLong {
        max: usize,
    },
    /// A line break, tab or other control character.
    Control,
}

impl Title {
    /// `text` without leading and trailing white space, if that is a valid title.
    ///
    /// # Errors
    /// The rule `text` breaks.
    pub fn new(text: &str) -> Result<Self, TitleError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(TitleError::Empty);
        }
        if text.chars().count() > MAX_TITLE_CHARS {
            return Err(TitleError::TooLong {
                max: MAX_TITLE_CHARS,
            });
        }
        if text.chars().any(char::is_control) {
            return Err(TitleError::Control);
        }
        Ok(Self(text.to_owned()))
    }

    /// "Untitled Artwork".
    #[must_use]
    pub fn untitled() -> Self {
        Self(UNTITLED.to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the gallery's search for `query` finds this title: it holds the query's text,
    /// trimmed, in any case; an empty query finds every title.
    #[must_use]
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty() || self.0.to_lowercase().contains(&query)
    }

    /// This title followed by `note` in brackets ("Harbour (5 Oct 2026 14:32)"), the title cut
    /// so the whole stays within `MAX_TITLE_CHARS`.
    #[must_use]
    pub fn noted(&self, note: &str) -> Self {
        let note = format!(" ({note})");
        let room = MAX_TITLE_CHARS.saturating_sub(note.chars().count());
        let kept: String = self.0.chars().take(room).collect();
        Self::new(&format!("{}{note}", kept.trim_end())).unwrap_or_else(|_| self.clone())
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for TitleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a title cannot be empty"),
            Self::TooLong { max } => write!(f, "a title has at most {max} characters"),
            Self::Control => f.write_str("a title is one line of text"),
        }
    }
}

impl std::error::Error for TitleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_trimmed_single_lines_of_limited_length() {
        assert_eq!(Title::new("  Harbour  ").map(|t| t.0), Ok("Harbour".into()));
        assert_eq!(Title::new(" \t "), Err(TitleError::Empty));
        assert_eq!(Title::new("a\nb"), Err(TitleError::Control));
        let long = "墨".repeat(MAX_TITLE_CHARS);
        assert!(Title::new(&long).is_ok(), "characters, not bytes");
        assert_eq!(
            Title::new(&format!("{long}x")),
            Err(TitleError::TooLong {
                max: MAX_TITLE_CHARS
            })
        );
        assert_eq!(Title::untitled().as_str(), UNTITLED);
    }

    #[test]
    fn the_search_finds_titles_holding_the_text_in_any_case() {
        let title = Title::new("Harbour at Dusk 墨").expect("title");
        for found in ["", "  ", "harbour", "AT DUSK", " dusk ", "墨", "r a"] {
            assert!(title.matches(found), "{found:?}");
        }
        for missed in ["harbor", "dusk!", "Hafen", "x"] {
            assert!(!title.matches(missed), "{missed:?}");
        }
    }

    #[test]
    fn a_noted_title_keeps_its_note_within_the_limit() {
        let title = Title::new("Harbour").expect("title");
        assert_eq!(
            title.noted("5 Oct 2026 14:32").as_str(),
            "Harbour (5 Oct 2026 14:32)"
        );
        let long = Title::new(&"a".repeat(MAX_TITLE_CHARS)).expect("long");
        let noted = long.noted("5 Oct 2026 14:32");
        assert_eq!(noted.as_str().chars().count(), MAX_TITLE_CHARS);
        assert!(noted.as_str().ends_with("a (5 Oct 2026 14:32)"));
    }
}
