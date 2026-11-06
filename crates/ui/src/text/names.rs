//! The names the app gives, shown in the language. The app stores them as it made them, in
//! English ("Layer 3", "Group 2", "Inserted image", "Pasted layer", "Untitled Artwork", the
//! built-in brushes' names, "<name> copy", a colour copy's "Colour copy 2", a picture of one's
//! own "Picture 4", a whole copy's "Division", "Line drawing" and "Colour blocks" layers), so they
//! read in whichever language shows and the stored name stays the control's name. A name the user typed shows as typed; one that reads like a default shows as
//! the default would.

use std::borrow::Cow;

use super::{BUILT_IN, Language, STUDY_EXERCISE};

texts! { NAMES;
    LAYER_NAME = "Layer", "图层";
    GROUP_NAME = "Group", "组";
    PICTURE_NAME = "Picture", "图片";
    INSERTED_IMAGE = "Inserted image", "插入的图片";
    PASTED_LAYER = "Pasted layer", "粘贴的图层";
    UNTITLED_ARTWORK = "Untitled Artwork", "未命名作品";
    COPY_NAME = "copy", "副本";
    DIVISION_LAYER_NAME = "Division", "画面分割";
    LINES_LAYER_NAME = "Line drawing", "线稿";
    BLOCKS_LAYER_NAME = "Colour blocks", "色块";
}

/// `name` as `language` shows it.
#[must_use]
pub(crate) fn shown_name(language: Language, name: &str) -> Cow<'_, str> {
    if language == Language::English {
        return Cow::Borrowed(name);
    }
    if let Some(base) = name.strip_suffix(" copy") {
        let copy = COPY_NAME.get(language);
        return Cow::Owned(format!("{} {copy}", shown_name(language, base)));
    }
    for word in [LAYER_NAME, GROUP_NAME, PICTURE_NAME, STUDY_EXERCISE] {
        if let Some(number) = name
            .strip_prefix(word.id())
            .and_then(|rest| rest.strip_prefix(' '))
            && !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Cow::Owned(format!("{} {number}", word.get(language)));
        }
    }
    let fixed = [
        INSERTED_IMAGE,
        PASTED_LAYER,
        UNTITLED_ARTWORK,
        DIVISION_LAYER_NAME,
        LINES_LAYER_NAME,
        BLOCKS_LAYER_NAME,
    ];
    match fixed.iter().chain(&BUILT_IN).find(|text| text.id() == name) {
        Some(text) => Cow::Borrowed(text.get(language)),
        None => Cow::Borrowed(name),
    }
}

#[cfg(test)]
mod tests {
    use openpaint_app::{BrushLibrary, Title};
    use openpaint_core::{Artwork, CanvasSize};

    use super::*;

    #[test]
    fn the_app_s_names_read_in_chinese_and_the_user_s_as_typed() {
        let zh = |name| shown_name(Language::Chinese, name).into_owned();
        assert_eq!(zh("Layer 12"), "图层 12");
        assert_eq!(zh("Group 3"), "组 3");
        assert_eq!(zh("Pencil copy copy"), "铅笔 副本 副本");
        assert_eq!(zh("Untitled Artwork"), "未命名作品");
        assert_eq!(zh("Colour copy 2"), "色彩临摹 2");
        assert_eq!(zh("Picture 4"), "图片 4");
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
        assert_eq!(shown_name(Language::English, "Layer 2"), "Layer 2");
    }

    #[test]
    fn the_names_are_those_the_app_and_its_brush_files_give() {
        assert_eq!(Title::untitled().as_str(), UNTITLED_ARTWORK.id());
        let canvas = CanvasSize::new(8, 8).expect("size");
        let artwork = Artwork::new(canvas);
        let layer = &artwork.layers()[0];
        assert_ne!(
            shown_name(Language::Chinese, layer.name().as_str()),
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
        // A whole copy's layers (S95).
        assert_eq!(DIVISION_LAYER_NAME.id(), openpaint_app::DIVISION_LAYER);
        assert_eq!(LINES_LAYER_NAME.id(), openpaint_app::LINES_LAYER);
        assert_eq!(BLOCKS_LAYER_NAME.id(), openpaint_app::BLOCKS_LAYER);
    }
}
