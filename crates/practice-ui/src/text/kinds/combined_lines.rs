//! Combined lines' words: their name and their How card, in V大's terms
//! (组合线条, 关键点, 叠图对比).

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("combined", COMBINED_LINES, COMBINED_HOW)];

const COMBINED_HOW: How = How {
    heading: COPY_COMBINED,
    trains: Some(GROUP_TRAINS),
    steps: &[GROUP_STEP_1, GROUP_STEP_2, GROUP_STEP_3, COMPARE_STEP],
    says: &[GROUP_SAYS],
};

texts! { PRACTICE_COMBINED_LINES;
    COMBINED_LINES = "Combined lines", "组合线条";
    COPY_COMBINED = "Copy combined lines", "临摹组合线条";
    GROUP_TRAINS = "Several lines seen as one shape: the key points first, then each \
        line.", "几根线看成一个形状：先定关键点，再画每根线。";
    GROUP_STEP_1 = "Key points with Sketch: ends, corners, meetings",
        "用草稿笔定关键点：端点、转角、交接处";
    GROUP_STEP_2 = "The longest line first, with Draw", "用勾线笔先画最长的线";
    GROUP_STEP_3 = "Each other from where it meets, at its angle", "其余的线从交接处按角度画出";
    GROUP_SAYS = "Compare, don't guess: the group lies over yours in red. In place shows where it \
        lies, Aligned its shape alone. Correct in blue and compare again.",
        "别凭感觉，要对比：整组原线条用红色叠在你的线上。“原位”看位置，“对齐”只比形状。用修正笔（蓝色）改，再对比。";
}
