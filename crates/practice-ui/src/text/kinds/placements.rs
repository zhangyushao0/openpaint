//! Placement's words (S153): its name and its How card, after V大's distribution method in our
//! own words (weeks 7 and 8): the plan seen from above read first, the ground divided by its
//! diagonals, each footprint's corners found there, the heights pulled up from the upright along
//! lines to the horizon, the tops closed. In his terms: 摆放, 平面图, 正视图, 对角线, 二等分,
//! 四等分, 视平线, 竖线.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("placement", PLACEMENT_EXERCISE, PLACEMENT)];

const PLACEMENT: How = How {
    heading: PLACEMENT_HEADING,
    trains: Some(PLACEMENT_TRAINS),
    steps: &[
        PLACEMENT_STEP_1,
        PLACEMENT_STEP_2,
        PLACEMENT_STEP_3,
        PLACEMENT_STEP_4,
        COMPARE_STEP,
    ],
    says: &[PLACEMENT_SAYS],
};

texts! { PRACTICE_PLACEMENTS;
    PLACEMENT_EXERCISE = "Placement", "摆放";
    PLACEMENT_HEADING = "Objects placed by a plan", "按平面图摆放物体";
    PLACEMENT_TRAINS = "Several objects put on the ground where a plan seen from above says, at \
        their heights: the ground divided first, then each object's corners, then its height \
        pulled up.",
        "把几个物体按俯视的平面图摆到透视的地面上，高度也要对：先分地面，再定每个物体的角，最后拉出高度。";
    PLACEMENT_STEP_1 = "Halves and quarters of the ground by its diagonals",
        "用对角线把地面分成二等分、四等分";
    PLACEMENT_STEP_2 = "Each object's corners where the plan has them",
        "按平面图在地面上定出每个物体的角";
    PLACEMENT_STEP_3 = "Each height from the upright along lines to the horizon",
        "从竖线沿着连到视平线的线拉出每个高度";
    PLACEMENT_STEP_4 = "The uprights and the tops with Draw", "用勾线笔画出竖边，封上顶面";
    PLACEMENT_SAYS = "Read the plan first: where each object stands, and in the front view how \
        tall it is against the unit. For a height, run a line from the upright's foot through the \
        object's nearest corner to the horizon, then one from that height on the upright to the \
        same point: it crosses the object's upright at its top. With no upright, stand one by \
        eye. Redraw a wrong one once with Correct.",
        "先读平面图：每个物体站在哪里；再在正视图里对着单位长度看它有多高。拉高度时，从竖线底端经过物体最近的角连到视平线，再从竖线上同样的高度连到这一点：它和物体竖边的交点就是顶。没给竖线时，凭眼睛立一根。画错了用修正笔重画一次。";
}
