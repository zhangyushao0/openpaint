//! An ellipse's words (week 2, for week 3's cylinders): its name and its How
//! card, after Drawabox (an ellipse through its frame's middles, ghosted first) and V大 (in
//! perspective a side's middle lies nearer its far end). In his terms: 椭圆, 对角线, 叠图对比.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("ellipse", ELLIPSE_EXERCISE, ELLIPSE_HOW)];

const ELLIPSE_HOW: How = How {
    heading: DRAW_AN_ELLIPSE,
    trains: Some(ELLIPSE_TRAINS),
    steps: &[
        ELLIPSE_STEP_1,
        ELLIPSE_STEP_2,
        ELLIPSE_STEP_3,
        ELLIPSE_STEP_4,
        COMPARE_STEP,
    ],
    says: &[ELLIPSE_SAYS],
};

texts! { PRACTICE_ELLIPSES;
    ELLIPSE_EXERCISE = "Ellipse", "椭圆";
    DRAW_AN_ELLIPSE = "Draw an ellipse in its frame", "在方框里画椭圆";
    ELLIPSE_TRAINS = "Week 2, for week 3's cylinders · an ellipse that fits a square in \
        perspective.", "第 2 周，为第 3 周的圆柱打基础 · 切于透视方形的椭圆。";
    ELLIPSE_STEP_1 = "Cross the diagonals: the centre", "连对角线找到中心";
    ELLIPSE_STEP_2 = "Through it, the four sides' middles", "过中心找到四条边的中点";
    ELLIPSE_STEP_3 = "Two passes in the air", "在空中比划两遍";
    ELLIPSE_STEP_4 = "One stroke through the middles, from the shoulder",
        "从肩膀带动，一笔画过四个中点";
    ELLIPSE_SAYS = "In perspective a side's middle lies nearer its far end. One stroke through the \
        middles; a wrong one redrawn once with Correct, never scratched over. Compare: the exact \
        ellipse in red, and a ring on each side's middle yours falls short of or runs past by \
        more than a pen's width. Construction shows the diagonals and middle lines. Compare again \
        after a correction: it reads your correction alone.",
        "透视中，边的中点更靠近远的一端。一笔画过四个中点；画错了用修正笔重画一次，不要来回描。叠图对比：红色是准确的椭圆；你的椭圆没碰到或超出某条边的中点一个笔宽以上，那个中点就画上红圈。点“看画法”，可以看到对角线和中线。修正后再对比，只看修正的那一笔。";
}
