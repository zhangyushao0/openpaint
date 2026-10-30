//! Divisions and whole copies on a practice sheet (S95, V大's weeks 3 and 4 assignment): the
//! kinds' names, the levels' pens, the card's table and the summary's lines; the How cards are
//! `practice_demo_division.rs`. In his terms: 画面分割, 主分割, 次分割, 三级, 整图复制.

use openpaint_practice::Level;

texts! { PRACTICE_DIVISION;
    DIVISION_EXERCISE = "Division", "画面分割";
    WHOLE_COPY_EXERCISE = "Whole copy", "整图复制";
    DIVIDE_A_PICTURE = "Divide a picture", "画面分割";
    COPY_A_WHOLE_PICTURE = "Copy a whole picture", "整图复制";
    MAIN_PEN = "Main", "主分割";
    SECOND_PEN = "Second", "次分割";
    THIRD_PEN = "Third", "三级";
    LINES_DRAWN = "Lines", "条数";
    MEAN_LENGTH = "Length", "平均长度";
    DIVISIONS_COUNTED = "Divisions", "画面分割";
    WHOLE_COPIES_COUNTED = "Whole copies", "整图复制";
    DRILL_NEEDS_COMPOSITIONS = "Divisions and whole copies use the composition pack: import it \
        under Practice packs.", "画面分割和整图复制用的是构图图包：请在“练习图包”下导入。";
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
