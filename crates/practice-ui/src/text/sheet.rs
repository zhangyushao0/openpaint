//! A practice sheet: the bar, the exercise's own tools and the card (each kind's words and How in
//! `kinds/`, a lesson's names, results and end in `course.rs`), in V大's terms: 叠图对比, 原位,
//! 对齐; 热身.

use openpaint_practice_app::Pen;

use super::Text;

texts! { PRACTICE_TABLE;
    HOW = "How", "怎么练";
    COMPARE = "Compare", "叠图对比";
    NEXT = "Next", "下一个";
    FINISH = "Finish", "结束";
    ONE_MORE = "One more", "再来一课";
    ALL_TOOLS = "All tools", "全部工具";
    SKETCH = "Sketch", "草稿笔";
    DRAW = "Draw", "勾线笔";
    CORRECT = "Correct", "修正笔";
    COLOUR_PICKED = "Colour picked", "当前颜色";
    FILL_FRAME = "Fill frame", "填满方框";
    GOT_IT = "Got it", "知道了";
    IN_PLACE = "In place", "原位";
    ALIGNED = "Aligned", "对齐";
    WARM_UP = "Warm-up", "热身";
    NO_OUTLINE = "This picture has no outline to align.", "这张图没有外轮廓，无法对齐。";
    ALIGNING = "Aligning…", "正在对齐…";
}

texts! { PRACTICE_SHEET_NOTICES;
    DRAW_IN_FRAME = "Draw in the empty frame.", "只能在空白框里画。";
    DRAW_IN_CELL = "Paint in a cell between the two ends.", "只能在两端之间的格子里画。";
    PICK_AFTER_COMPARE = "The original can be picked after Compare.", "叠图对比之后才能吸原图的颜色。";
    SESSION_DONE = "This lesson is done.", "这一课已经上完。";
    DROP_IN_FRAME = "Drop inside the empty frame to fill.", "拖到空白框里松手即可填充。";
}

/// An exercise's pens.
pub(crate) const fn pen(pen: Pen) -> Text {
    match pen {
        Pen::Sketch => SKETCH,
        Pen::Draw => DRAW,
        Pen::Correct => CORRECT,
        Pen::Big => super::BIG,
        Pen::Small => super::SMALL,
        Pen::Soft => super::SOFT,
        Pen::Main => super::MAIN_PEN,
        Pen::Second => super::SECOND_PEN,
        Pen::Third => super::THIRD_PEN,
        Pen::Dark => super::DARK_PEN,
        Pen::Middle => super::MIDDLE_PEN,
        Pen::Light => super::LIGHT_PEN,
    }
}
