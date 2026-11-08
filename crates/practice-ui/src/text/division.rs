//! Divisions and whole copies on a practice sheet (S95, V大's weeks 3 and 4 assignment): the
//! levels' pens, the card's table and the summary's numbers; the kinds' names and How cards are
//! `kinds/`. In his terms: 画面分割, 主分割, 次分割, 三级分割, 整图复制.

use openpaint_practice::Level;

texts! { PRACTICE_DIVISION;
    MAIN_PEN = "Main", "主分割";
    SECOND_PEN = "Second", "次分割";
    THIRD_PEN = "Third", "三级分割";
    LINES_DRAWN = "Lines", "条数";
    MEAN_LENGTH = "Length", "平均长度";
    DRILL_NEEDS_COMPOSITIONS = "Divisions and whole copies use the composition pack: import it \
        under Practice packs.", "画面分割和整图复制要用构图素材包：请在“素材包”下导入。";
}

/// A level's pen's name, as its row on the card names it too.
pub(crate) const fn level(level: Level) -> crate::text::Text {
    match level {
        Level::Main => MAIN_PEN,
        Level::Second => SECOND_PEN,
        Level::Third => THIRD_PEN,
    }
}

/// How far a level's lines run on the mean: "78 %" of the picture's longer side.
pub(crate) fn mean_length(share: f64) -> String {
    format!("{:.0} %", share * 100.0)
}
