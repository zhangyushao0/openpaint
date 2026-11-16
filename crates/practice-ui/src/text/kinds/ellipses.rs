//! An ellipse's words (week 2, for week 3's cylinders): its name and its How
//! card, after Drawabox (an ellipse through its frame's middles, ghosted first) and V大 (in
//! perspective a side's middle lies nearer its far end). In his terms: 椭圆, 对角线, 叠图对比. A
//! funnel's (S156l, Drawabox's funnels, a topic of its own): the axis first, each ellipse round it,
//! touching both sides; 漏斗, 中轴, 短轴, 长轴.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 2] = [
    words("ellipse", ELLIPSE_EXERCISE, ELLIPSE_HOW),
    words("funnel", FUNNEL_EXERCISE, FUNNEL_HOW),
];

const FUNNEL_HOW: How = How {
    heading: FUNNEL_HEADING,
    trains: Some(FUNNEL_TRAINS),
    steps: &[FUNNEL_STEP_1, FUNNEL_STEP_2, FUNNEL_STEP_3, COMPARE_STEP],
    says: &[FUNNEL_SAYS],
};

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
    ELLIPSE_TRAINS = "An ellipse that fits a square in perspective, the base of the cylinders \
        to come.", "切于透视方形的椭圆，为之后的圆柱打基础。";
    ELLIPSE_STEP_1 = "Cross the diagonals: the centre", "连对角线找到中心";
    ELLIPSE_STEP_2 = "Through it, the four sides' middles", "过中心找到四条边的中点";
    ELLIPSE_STEP_3 = "Two passes in the air", "在空中比划两遍";
    ELLIPSE_STEP_4 = "One stroke through the middles, from the shoulder",
        "从肩膀带动，一笔画过四个中点";
    ELLIPSE_SAYS = "In perspective a side's middle lies nearer its far end. One stroke through the \
        middles; a wrong one redrawn once with Correct, never scratched over.",
        "透视中，边的中点更靠近远的一端。一笔画过四个中点；画错了用修正笔重画一次，不要来回描。";
    FUNNEL_EXERCISE = "Funnel", "漏斗";
    FUNNEL_HEADING = "Ellipses on one axis", "沿一条中轴画椭圆";
    FUNNEL_TRAINS = "Ellipses aligned on an axis that is not the hand's habit, each wide enough \
        to touch both sides of the funnel.",
        "沿一条不顺手的中轴对齐几个椭圆，每个都宽到碰到漏斗的两边。";
    FUNNEL_STEP_1 = "The axis first: each ellipse's long axis square to it",
        "先看中轴：每个椭圆的长轴都和它垂直";
    FUNNEL_STEP_2 = "Two passes in the air round the axis", "绕着中轴在空中比划两遍";
    FUNNEL_STEP_3 = "Each round its short axis, touching both sides",
        "每个椭圆围着它的短轴画，碰到两边";
    FUNNEL_SAYS = "The axis first: each ellipse's short axis lies on it, its long axis square to \
        it. Draw each through the ends of its short axis, wide enough to touch both sides and no \
        wider; its roundness changes along the funnel. Ghost it round the axis, then one stroke; \
        redraw a wrong one once with Correct.",
        "先看中轴：每个椭圆的短轴落在中轴上，长轴和中轴垂直。每个椭圆穿过它短轴的两端，宽到刚好碰到两边，不要更宽；沿着漏斗，椭圆的圆度会变。先绕着中轴在空中比划，再一笔画出；画错了用修正笔重画一次。";
}
