//! A single line's words: its name and its How card, in V大's terms (草稿笔,
//! 勾线笔, 修正笔, 叠图对比, 最鼓处).

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("line", LINE, LINE_HOW)];

const LINE_HOW: How = How {
    heading: COPY_A_LINE,
    trains: Some(LINE_TRAINS),
    steps: &[LINE_STEP_1, LINE_STEP_2, LINE_STEP_3, COMPARE_STEP],
    says: &[LINE_SAYS],
};

texts! { PRACTICE_LINES;
    LINE = "Line", "线条";
    COPY_A_LINE = "Copy a line", "临摹单根线条";
    LINE_TRAINS = "A line placed by its ends and bends, then drawn in one sure stroke.",
        "先定两端和转折，再果断地一笔画出。";
    LINE_STEP_1 = "Tick its ends and where it bulges most with Sketch",
        "用草稿笔标出两端和最鼓处";
    LINE_STEP_2 = "Two passes in the air", "在空中比划两遍";
    LINE_STEP_3 = "One quick stroke with Draw", "用勾线笔一笔画出";
    LINE_SAYS = "Don't erase: redraw over it with Correct and compare again.",
        "不要擦，用修正笔直接在上面改，再对比。";
}
