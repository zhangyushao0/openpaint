//! The names practice gives, stored in English and shown in the language through the ui's names
//! (`openpaint_ui::text::Names`, `NAMES`): a colour copy's "Colour copy 2", a picture of one's own
//! "Picture 4", a whole copy's "Division", "Line drawing" and "Colour blocks" layers; and the
//! official packs, named by their id.

use super::{Language, Names, STUDY_EXERCISE};

texts! { PRACTICE_NAMES;
    PICTURE_NAME = "Picture", "图片";
    DIVISION_LAYER_NAME = "Division", "画面分割";
    LINES_LAYER_NAME = "Line drawing", "线稿";
    BLOCKS_LAYER_NAME = "Colour blocks", "色块";
    SHAPES_PACK = "Single shapes", "单体图形";
    COMPOSITIONS_PACK = "Compositions", "构图";
    COMBINATIONS_PACK = "Colour combinations", "色彩组合";
}

/// The names practice gives, for the shell (`Shell::set_names`).
pub const NAMES: Names = Names {
    numbered: &[PICTURE_NAME, STUDY_EXERCISE],
    fixed: &[DIVISION_LAYER_NAME, LINES_LAYER_NAME, BLOCKS_LAYER_NAME],
};

/// The official packs by their id, each with its name (S102b).
const OFFICIAL_PACKS: [(&str, super::Text); 3] = [
    ("openpaint-shapes", SHAPES_PACK),
    ("openpaint-compositions", COMPOSITIONS_PACK),
    ("openpaint-combinations", COMBINATIONS_PACK),
];

/// A pack's name as `language` shows it: an official pack's from the table by its `id`, any
/// other's `title` as its file holds it.
#[must_use]
pub(crate) fn pack_title<'a>(language: Language, id: &str, title: &'a str) -> &'a str {
    OFFICIAL_PACKS
        .iter()
        .find(|(official, _)| *official == id)
        .map_or(title, |(_, name)| name.get(language))
}

#[cfg(test)]
mod tests {
    use openpaint_ui::text::{set_language, set_names, shown_name};

    use super::*;

    #[test]
    fn practice_s_names_read_in_chinese() {
        let ctx = egui::Context::default();
        set_language(&ctx, Language::Chinese);
        set_names(&ctx, NAMES);
        let zh = |name| shown_name(&ctx, name).into_owned();
        assert_eq!(zh("Colour copy 2"), "色彩临摹 2");
        assert_eq!(zh("Picture 4"), "图片 4");
        assert_eq!(zh("Division"), "画面分割");
        assert_eq!(zh("Line drawing copy"), "线稿 副本");
        assert_eq!(zh("Picture"), "Picture", "no number");
    }

    #[test]
    fn a_whole_copy_s_layers_are_named_as_the_flow_names_them() {
        assert_eq!(
            DIVISION_LAYER_NAME.id(),
            openpaint_practice_app::DIVISION_LAYER
        );
        assert_eq!(LINES_LAYER_NAME.id(), openpaint_practice_app::LINES_LAYER);
        assert_eq!(BLOCKS_LAYER_NAME.id(), openpaint_practice_app::BLOCKS_LAYER);
    }

    #[test]
    fn official_packs_are_named_in_the_language_and_others_as_their_files() {
        let zh = |id, title| pack_title(Language::Chinese, id, title);
        assert_eq!(zh("openpaint-shapes", "Single shapes"), "单体图形");
        assert_eq!(zh("openpaint-compositions", "Compositions"), "构图");
        assert_eq!(
            zh("openpaint-combinations", "Colour combinations"),
            "色彩组合"
        );
        let en = pack_title(Language::English, "openpaint-compositions", "Old title");
        assert_eq!(en, "Compositions");
        assert_eq!(zh("someone-else", "Harbour studies"), "Harbour studies");
    }
}
