//! Every word the interface shows, in English and Chinese (DECISIONS D100). A `Text` holds both,
//! side by side, so a missing translation does not compile and each Chinese line sits beside the
//! English it renders; the tables are split by where the words show (`common.rs`, `canvas.rs`,
//! `layers.rs`, `brushes.rs`, `colour.rs`, `gallery.rs`, `notices.rs`, `practice.rs`,
//! `practice_colour.rs`, `practice_options.rs`, `practice_perspective.rs`, `practice_points.rs`,
//! `practice_book.rs`), words made with numbers are functions beside them. Screens write no words
//! of their own (`cargo repo-check` refuses a capitalised literal under `crates/ui/src` outside
//! this module).
//!
//! The English is a control's name too: the control map registers it whichever language shows
//! (D48), so scenarios, benches and device chores name controls alike in both. A name the user
//! gave shows as given; the names the app gives by default show in the language (`names.rs`).
//! The shell sets the language for the pass on the egui context; components read it.

/// A table: each line `NAME = "English", "中文";` makes a `Text` constant, and `$all` lists them
/// for the tables' tests.
macro_rules! texts {
    ($all:ident; $($name:ident = $en:literal, $zh:literal;)*) => {
        $(pub(crate) const $name: crate::text::Text = crate::text::Text::new($en, $zh);)*
        /// Every text of this table, for the tables' tests.
        #[cfg(test)]
        pub(crate) const $all: &[crate::text::Text] = &[$($name),*];
    };
}

mod brushes;
mod canvas;
mod colour;
mod common;
mod gallery;
mod layers;
mod names;
mod notices;
mod practice;
mod practice_book;
mod practice_colour;
mod practice_options;
mod practice_perspective;
mod practice_points;

pub(crate) use brushes::*;
pub(crate) use canvas::*;
pub(crate) use colour::*;
pub(crate) use common::*;
pub(crate) use gallery::*;
pub(crate) use layers::*;
pub(crate) use names::*;
pub(crate) use notices::*;
pub(crate) use practice::*;
pub(crate) use practice_book::*;
pub(crate) use practice_colour::*;
pub(crate) use practice_options::*;
pub(crate) use practice_perspective::*;
pub(crate) use practice_points::*;

pub use openpaint_app::Language;

use egui::Id;

/// Where the pass's language is kept in egui's memory.
const STATE: &str = "language";

/// Words the interface shows, in both languages; the English names the control they label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    en: &'static str,
    zh: &'static str,
}

impl Text {
    pub(crate) const fn new(en: &'static str, zh: &'static str) -> Self {
        Self { en, zh }
    }

    /// The English, which also names the control.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.en
    }

    /// As `language` reads it.
    #[must_use]
    pub const fn get(self, language: Language) -> &'static str {
        match language {
            Language::English => self.en,
            Language::Chinese => self.zh,
        }
    }
}

/// What a component shows and names its control by: a table's `Text`, or words made for the
/// language shown (a name, a number) with the name the control goes by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Words<'a> {
    id: &'a str,
    en: &'a str,
    zh: &'a str,
}

impl Words<'static> {
    /// No words (a row without a note).
    pub const EMPTY: Self = Self::made("");
}

impl<'a> Words<'a> {
    /// Words already in the language shown (made by a table's function, or the user's), named
    /// by themselves.
    #[must_use]
    pub const fn made(shown: &'a str) -> Self {
        Self::named(shown, shown)
    }

    /// `shown`, named `id` whichever language shows (a layer's stored name, shown in the
    /// language when the app gave it).
    #[must_use]
    pub const fn named(id: &'a str, shown: &'a str) -> Self {
        Self {
            id,
            en: shown,
            zh: shown,
        }
    }

    /// The control's name.
    #[must_use]
    pub const fn id(&self) -> &'a str {
        self.id
    }

    /// As `language` reads them.
    #[must_use]
    pub const fn get(&self, language: Language) -> &'a str {
        match language {
            Language::English => self.en,
            Language::Chinese => self.zh,
        }
    }

    /// As the pass's language reads them.
    #[must_use]
    pub fn shown(&self, ctx: &egui::Context) -> &'a str {
        self.get(language(ctx))
    }
}

impl From<Text> for Words<'_> {
    fn from(text: Text) -> Self {
        Self {
            id: text.en,
            en: text.en,
            zh: text.zh,
        }
    }
}

/// The pass's language, as the shell set it.
pub(crate) fn language(ctx: &egui::Context) -> Language {
    ctx.data(|data| data.get_temp(Id::new(STATE)))
        .unwrap_or_default()
}

/// The language this pass shows.
pub(crate) fn set_language(ctx: &egui::Context, language: Language) {
    ctx.data_mut(|data| data.insert_temp(Id::new(STATE), language));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_reads_in_each_language_and_is_named_in_english() {
        let words = Words::from(UNDO);
        assert_eq!(words.id(), "Undo");
        assert_eq!(words.get(Language::English), "Undo");
        assert_eq!(words.get(Language::Chinese), "撤销");
        let layer = Words::named("Layer 2", "图层 2");
        assert_eq!(
            (layer.id(), layer.get(Language::English)),
            ("Layer 2", "图层 2")
        );
        let ctx = egui::Context::default();
        assert_eq!(language(&ctx), Language::English);
        set_language(&ctx, Language::Chinese);
        assert_eq!(Words::from(REDO).shown(&ctx), "重做");
    }

    #[test]
    fn every_text_reads_in_both_languages_and_the_chinese_in_chinese() {
        let tables = [
            COMMON,
            CANVAS,
            CANVAS_WORDS,
            LAYERS_TABLE,
            LAYER_WORDS,
            BRUSHES_TABLE,
            BRUSH_WORDS,
            BRUSH_NAMES,
            COLOUR_TABLE,
            GALLERY_TABLE,
            GALLERY_NOTICES,
            NAMES,
            NOTICES,
            PRACTICE_TABLE,
            PRACTICE_STEPS,
            PRACTICE_BOOK_TABLE,
            PRACTICE_BOOK_WEEKDAYS,
            PRACTICE_COLOUR,
            PRACTICE_COLOUR_STEPS,
            PRACTICE_OPTIONS_TABLE,
            PRACTICE_PERSPECTIVE,
            PRACTICE_PERSPECTIVE_STEPS,
            PRACTICE_POINTS,
            PRACTICE_POINTS_STEPS,
        ];
        let han = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
        for text in tables.concat() {
            let (english, chinese) = (text.id(), text.get(Language::Chinese));
            assert!(!english.is_empty() && !chinese.is_empty(), "{english}");
            assert!(
                chinese.chars().any(han) || text == ENGLISH,
                "{english}: {chinese}"
            );
        }
    }
}
