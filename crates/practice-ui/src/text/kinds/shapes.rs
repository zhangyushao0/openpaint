//! A single shape's words: its name and its How card, in V大's terms (单体图形,
//! 外轮廓, 负形, 叠图对比).

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("shape", SHAPE_EXERCISE, SHAPE_HOW)];

const SHAPE_HOW: How = How {
    heading: COPY_A_SHAPE,
    trains: Some(SHAPE_TRAINS),
    steps: &[
        SHAPE_STEP_1,
        SHAPE_STEP_2,
        SHAPE_STEP_3,
        SHAPE_STEP_4,
        COMPARE_STEP,
    ],
    says: &[SHAPE_SAYS],
};

texts! { PRACTICE_SHAPES;
    SHAPE_EXERCISE = "Shape", "单体图形";
    COPY_A_SHAPE = "Copy a single shape", "临摹单体图形";
    SHAPE_TRAINS = "An object's outline as a line drawing: big shapes before \
        small turns, checked by the negative shapes.",
        "用线条画物体的外轮廓：先大后小，用负形检查。";
    SHAPE_STEP_1 = "Tick its top, bottom, left and right", "标出最上、最下、最左、最右的点";
    SHAPE_STEP_2 = "Block it in with long straight lines", "用长直线概括大形";
    SHAPE_STEP_3 = "Check a negative shape: the ground beside it", "检查负形：旁边的空白";
    SHAPE_STEP_4 = "The outline with Draw", "用勾线笔画出外轮廓";
    SHAPE_SAYS = "Compare lays the picture's outline over yours in red, the picture faint under \
        it: In place shows where you placed it, Aligned its shape alone. Correct in blue and \
        compare again.",
        "叠图对比把原图的外轮廓用红色叠在你的线上，下面淡淡的是原图：“原位”看你放的位置，“对齐”只比形状。用修正笔（蓝色）改，再对比。";
}
