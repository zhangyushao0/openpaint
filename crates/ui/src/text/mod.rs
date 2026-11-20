//! Every word the interface shows, in English and Chinese (DECISIONS D100). A `Text` holds both,
//! side by side, so a missing translation does not compile and each Chinese line sits beside the
//! English it renders; the tables are split by where the words show (`common.rs`, `canvas.rs`,
//! `layers.rs`, `brushes.rs`, `colour.rs`, `gallery.rs`, `notices.rs`, `replay.rs`), words made
//! with numbers are functions beside them. A mode's screens keep their words in tables of their
//! own crate, made with the same `texts!` and `Text`, beside these (D112). Screens write no words
//! of their own (`cargo repo-check` refuses a capitalised literal under `crates/ui/src` outside
//! this module).
//!
//! The English is a control's name too: the control map registers it whichever language shows
//! (D48), so scenarios, benches and device chores name controls alike in both. A name the user
//! gave shows as given; the names the app and a mode give by default show in the language
//! (`names.rs`). The shell sets the language and the mode's names for the pass on the egui
//! context; components read them.
//!
//! The Chinese is written Chinese-first (S102, D100): as a Chinese art teacher says it, short, a
//! command from its verb, a hint saying what to do; one Chinese term for each English one in
//! every table: V大's words for the course's terms (原图 the picture copied, never 原色, a primary
//! colour; 明度, 纯度, 色块, 勾线笔, 素材包), Procreate's for the painting tools unless the author
//! chose (压感, 防抖, 色环, 变换, 叠加). A space between Han and Latin letters or digits ("第 2 周",
//! "保存 PNG…"), none in dates (2026年10月5日) or after V大; numbers in the app's units (50 %,
//! 90°); a control named in a sentence in “”. A term dropped joins `RETIRED`.

/// A table: each line `NAME = "English", "中文";` makes a `Text` constant, and `$all` lists them
/// for the tables' tests.
#[macro_export]
macro_rules! texts {
    ($all:ident; $($name:ident = $en:literal, $zh:literal;)*) => {
        $(pub const $name: $crate::text::Text = $crate::text::Text::new($en, $zh);)*
        /// Every text of this table, for the tables' tests.
        #[cfg(test)]
        pub const $all: &[$crate::text::Text] = &[$($name),*];
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
mod replay;

pub use brushes::*;
pub use canvas::*;
pub use colour::*;
pub use common::*;
pub use gallery::*;
pub use layers::*;
pub use names::*;
pub use notices::*;
pub use replay::*;

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
    /// The English and the Chinese of the same words (a table's line, `texts!`).
    #[must_use]
    pub const fn new(en: &'static str, zh: &'static str) -> Self {
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
#[must_use]
pub fn language(ctx: &egui::Context) -> Language {
    ctx.data(|data| data.get_temp(Id::new(STATE)))
        .unwrap_or_default()
}

/// The language this pass shows.
pub fn set_language(ctx: &egui::Context, language: Language) {
    ctx.data_mut(|data| data.insert_temp(Id::new(STATE), language));
}

/// Terms the Chinese no longer uses (S102), each with what took its place: no table holds them,
/// a mode's neither (its tables' tests).
pub const RETIRED: [(&str, &str); 13] = [
    ("原色", "原图的颜色: 原色 is a primary colour"),
    ("画图笔", "勾线笔 (Draw)"),
    ("图包", "素材包"),
    ("色轮", "色环"),
    ("色片", "色卡"),
    (
        "笔画",
        "笔触 (a stroke drawn); 描边 (the brush editor's page)",
    ),
    ("外形轮廓", "外轮廓, or 轮廓 on a label"),
    ("图像", "图片"),
    ("压力", "压感"),
    ("流线", "防抖"),
    ("等距延伸", "等距扩展"),
    ("等比分割", "按比例找点"),
    ("原点", "已知点: 原点 is the origin"),
];

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

    /// Every table's texts.
    fn every_text() -> Vec<Text> {
        [
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
            REPLAY_TABLE,
        ]
        .concat()
    }

    #[test]
    fn every_text_reads_in_both_languages_and_the_chinese_in_chinese() {
        let han = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
        for text in every_text() {
            let (english, chinese) = (text.id(), text.get(Language::Chinese));
            assert!(!english.is_empty() && !chinese.is_empty(), "{english}");
            assert!(
                chinese.chars().any(han) || text == ENGLISH,
                "{english}: {chinese}"
            );
        }
    }

    #[test]
    fn the_chinese_holds_no_retired_term() {
        use openpaint_app::ImageError;
        let zh = Language::Chinese;
        // A brush's name then 画 (draws), and 一笔画 (one stroke), are no 笔画.
        let drawing: Vec<String> = (BUILT_IN.into_iter())
            .map(|brush| format!("{}画", brush.get(zh)))
            .chain(["一笔画".to_owned()])
            .collect();
        let made = [
            ImageError::Unsupported,
            ImageError::Damaged,
            ImageError::TooLarge,
        ]
        .map(|error| image_failed(zh, true, &error));
        let tables = every_text().into_iter().map(|text| text.get(zh).to_owned());
        for chinese in tables.chain(made) {
            let read = (drawing.iter()).fold(chinese.clone(), |read, verb| read.replace(verb, ""));
            for (term, instead) in RETIRED {
                assert!(!read.contains(term), "{chinese}: {term}, now {instead}");
            }
        }
    }
}
