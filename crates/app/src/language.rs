//! The interface's language (DECISIONS D100): English or Chinese, chosen in Actions and stored
//! with the toolbox; until one is chosen, the system's. The words themselves are the UI's
//! (`openpaint-ui`'s text tables); the app knows only which language shows.

/// A language the interface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    English,
    /// Simplified Chinese.
    Chinese,
}

impl Language {
    /// As Actions offers them.
    pub const ALL: [Self; 2] = [Self::English, Self::Chinese];

    /// The language of the system's locale, a BCP 47 tag ("zh-Hans-CN", "zh-TW", "en-GB"):
    /// Chinese for any Chinese, English for everything else.
    #[must_use]
    pub fn of_tag(tag: &str) -> Self {
        let primary = tag.split(['-', '_']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("zh") {
            Self::Chinese
        } else {
            Self::English
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brushes::BrushLibrary;
    use crate::tool_intent::ToolIntent;
    use crate::toolbox::Toolbox;

    #[test]
    fn the_toolbox_keeps_the_language_chosen_and_none_until_then() {
        let mut toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
        assert_eq!(toolbox.language(), None, "the system's");
        assert_eq!(toolbox.save_request(), None);
        toolbox.dispatch_tool(&ToolIntent::Language(Language::Chinese));
        assert_eq!(toolbox.language(), Some(Language::Chinese));
        let stored = toolbox.save_request().expect("to store");
        assert_eq!(stored.language, Some(Language::Chinese));
        toolbox.dispatch_tool(&ToolIntent::Language(Language::Chinese));
        assert_eq!(
            toolbox.save_request(),
            None,
            "the same again: nothing to store"
        );
        let mut reopened = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
        assert!(reopened.restore(&stored));
        assert_eq!(reopened.language(), Some(Language::Chinese));
    }

    #[test]
    fn a_chinese_locale_is_chinese_and_everything_else_english() {
        for tag in ["zh", "zh-Hans-CN", "zh_CN", "ZH-tw", "zh-Hant-HK"] {
            assert_eq!(Language::of_tag(tag), Language::Chinese, "{tag}");
        }
        for tag in ["en", "en-GB", "de-DE", "ja-JP", "", "und", "zha"] {
            assert_eq!(Language::of_tag(tag), Language::English, "{tag}");
        }
    }
}
