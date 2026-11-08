//! The names the app gives, shown in the language. The app stores them as it made them, in
//! English ("Layer 3", "Group 2", "Inserted image", "Pasted layer", "Untitled Artwork", the
//! built-in brushes' names, "<name> copy"), so they read in whichever language shows and the
//! stored name stays the control's name; a mode gives names of its own alike (`Names`, set on
//! the shell, D112). A name the user typed shows as typed; one that reads like a default shows as
//! the default would.

use std::borrow::Cow;

use egui::Id;

use super::{BUILT_IN, Language, Text, language};

texts! { NAMES;
    LAYER_NAME = "Layer", "图层";
    GROUP_NAME = "Group", "组";
    INSERTED_IMAGE = "Inserted image", "插入的图片";
    PASTED_LAYER = "Pasted layer", "粘贴的图层";
    UNTITLED_ARTWORK = "Untitled Artwork", "未命名作品";
    COPY_NAME = "copy", "副本";
}

/// Where the pass's mode names are kept in egui's memory.
const STATE: &str = "mode names";

/// The names a mode gives its records and layers, stored in English and shown in the language: a
/// word then a number ("Picture 4"), or a name as it is ("Division").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Names {
    pub numbered: &'static [Text],
    pub fixed: &'static [Text],
}

/// The mode's names this pass shows in the language (the shell sets them, `Shell::set_names`).
pub fn set_names(ctx: &egui::Context, names: Names) {
    ctx.data_mut(|data| data.insert_temp(Id::new(STATE), names));
}

/// `name` as the pass's language shows it.
#[must_use]
pub fn shown_name<'a>(ctx: &egui::Context, name: &'a str) -> Cow<'a, str> {
    let names: Names = ctx
        .data(|data| data.get_temp(Id::new(STATE)))
        .unwrap_or_default();
    named((language(ctx), names), name)
}

/// `name` as `language` shows it, the mode giving `names`.
fn named(on: (Language, Names), name: &str) -> Cow<'_, str> {
    let (language, names) = on;
    if language == Language::English {
        return Cow::Borrowed(name);
    }
    if let Some(base) = name.strip_suffix(" copy") {
        let copy = COPY_NAME.get(language);
        return Cow::Owned(format!("{} {copy}", named(on, base)));
    }
    for word in [LAYER_NAME, GROUP_NAME].iter().chain(names.numbered) {
        if let Some(number) = name
            .strip_prefix(word.id())
            .and_then(|rest| rest.strip_prefix(' '))
            && !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Cow::Owned(format!("{} {number}", word.get(language)));
        }
    }
    let fixed = [INSERTED_IMAGE, PASTED_LAYER, UNTITLED_ARTWORK];
    let mut given = fixed.iter().chain(names.fixed).chain(&BUILT_IN);
    match given.find(|text| text.id() == name) {
        Some(text) => Cow::Borrowed(text.get(language)),
        None => Cow::Borrowed(name),
    }
}

#[cfg(test)]
mod tests {
    use openpaint_app::{BrushLibrary, Title};
    use openpaint_core::{Artwork, CanvasSize};

    use super::*;

    const NUMBERED: Text = Text::new("Sketch", "速写");
    const FIXED: Text = Text::new("Ground", "底色");

    /// A pass's context in Chinese, a mode giving "Sketch n" and "Ground".
    fn chinese() -> egui::Context {
        let ctx = egui::Context::default();
        super::super::set_language(&ctx, Language::Chinese);
        let names = Names {
            numbered: &[NUMBERED],
            fixed: &[FIXED],
        };
        set_names(&ctx, names);
        ctx
    }

    #[test]
    fn the_app_s_names_read_in_chinese_and_the_user_s_as_typed() {
        let ctx = chinese();
        let zh = |name| shown_name(&ctx, name).into_owned();
        assert_eq!(zh("Layer 12"), "图层 12");
        assert_eq!(zh("Group 3"), "组 3");
        assert_eq!(zh("Pencil copy copy"), "铅笔 副本 副本");
        assert_eq!(zh("Untitled Artwork"), "未命名作品");
        assert_eq!(zh("Sketch 2"), "速写 2", "a mode's numbered name");
        assert_eq!(zh("Ground copy"), "底色 副本", "a mode's fixed name");
        for kept in [
            "Layer",
            "Layer x",
            "Layers 2",
            "Harbour",
            "我的图层",
            "Layer 2b",
        ] {
            assert_eq!(zh(kept), kept);
        }
        let english = egui::Context::default();
        assert_eq!(shown_name(&english, "Layer 2"), "Layer 2");
        assert_eq!(shown_name(&english, "Sketch 2"), "Sketch 2");
    }

    #[test]
    fn the_names_are_those_the_app_and_its_brush_files_give() {
        assert_eq!(Title::untitled().as_str(), UNTITLED_ARTWORK.id());
        let canvas = CanvasSize::new(8, 8).expect("size");
        let artwork = Artwork::new(canvas);
        let layer = &artwork.layers()[0];
        let ctx = chinese();
        assert_ne!(
            shown_name(&ctx, layer.name().as_str()),
            layer.name().as_str()
        );
        let library = BrushLibrary::built_in().expect("brushes");
        for brush in library.brushes() {
            let name = brush.name();
            assert!(
                BUILT_IN.iter().any(|text| text.id() == name),
                "{name} has no Chinese"
            );
        }
        assert_eq!(library.brushes().len(), BUILT_IN.len());
    }
}
