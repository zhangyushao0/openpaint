//! Week 2's perspective How cards (`practice_demo.rs`), after V大's method in our own words (near
//! is bigger, far is smaller; edges running the same way meet at one vanishing point, those
//! parallel to the ground on the horizon; the front angle wider than 90 degrees; the hidden edges
//! drawn too) and Drawabox's (ghost a line before drawing it; an ellipse through its frame's
//! middles). In his terms: 透视盒子, 视平线, 消失点, 前角, 椭圆, 对角线, 叠图对比.

use super::practice_demo::{COMPARE_STEP, How};

pub(crate) const BOX: How = How {
    heading: super::COMPLETE_THE_BOX,
    trains: Some(BOX_TRAINS),
    steps: &[BOX_STEP_1, BOX_STEP_2, BOX_STEP_3, COMPARE_STEP],
    says: &[BOX_SAYS],
};
pub(crate) const FREE: How = How {
    heading: super::FREE_BOXES_EXERCISE,
    trains: Some(FREE_TRAINS),
    steps: &[
        FREE_STEP_1,
        FREE_STEP_2,
        FREE_STEP_3,
        FREE_STEP_4,
        COMPARE_STEP,
    ],
    says: &[FREE_SAYS],
};
pub(crate) const ELLIPSE: How = How {
    heading: super::DRAW_AN_ELLIPSE,
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

texts! { PRACTICE_DEMO_SPACE;
    BOX_TRAINS = "Week 2 · perspective: edges running one way meet at one vanishing point on the \
        horizon.", "第 2 周 · 透视：同向的边交于视平线上的同一消失点。";
    BOX_STEP_1 = "Run the given edges on to the horizon", "把给出的边延长到视平线";
    BOX_STEP_2 = "Back from the points to the far corners", "从消失点连回远处的角";
    BOX_STEP_3 = "The nine edges with Draw, hidden ones too", "画图笔画出九条边，看不见的也画";
    BOX_SAYS = "Ghost each edge before the pen goes down. Compare lays the exact box over yours in \
        red, its vanishing points ringed; each of your edges is drawn on dashed towards its point, \
        blue where it heads for it, red where it misses. Correct in blue, the correction counting \
        in place of the edge it redraws, and compare again.",
        "下笔前先空比划几次。叠图对比把红色的准确盒子叠在你的盒子上，消失点画了红圈；你的每条边用虚线朝它的消失点延长，指向消失点的是蓝色，偏离的是红色。用修正笔（蓝色）改，修正的边代替它重画的那条边，再对比。";
    FREE_TRAINS = "Week 2 · boxes of your own on a horizon, each turned its own way, every front \
        corner wider than 90°.", "第 2 周 · 视平线上自己画盒子，转向各异，前角都大于 90°。";
    FREE_STEP_1 = "Two points on the horizon", "视平线上定两个消失点";
    FREE_STEP_2 = "A front upright", "画一条靠前的竖边";
    FREE_STEP_3 = "Its edges towards the points", "从它两端连向消失点";
    FREE_STEP_4 = "The far uprights, then the back", "再画远处的竖边和后面的边";
    FREE_SAYS = "Draw three or four, each of other proportions; points too close distort a box. \
        Compare draws each straight edge on dashed to the horizon, blue where a direction's edges \
        meet (ringed), red where one misses; red also marks a leaning vertical's true vertical and \
        a front corner under 90°. Correct in blue and compare again.",
        "画三四个，长宽高比例各不相同；消失点太近盒子会畸变。叠图对比把每条直边用虚线延长到视平线，同一方向的边交于一点（画圈）的是蓝色，偏离的是红色；红色还标出不竖直的竖边的真正竖直线，以及小于 90° 的前角。用修正笔（蓝色）改，再对比。";
    ELLIPSE_TRAINS = "Week 2, for week 3's cylinders · an ellipse that fits a square in \
        perspective.", "第 2 周，为第 3 周的圆柱打基础 · 贴合透视方形的椭圆。";
    ELLIPSE_STEP_1 = "Cross the diagonals: the centre", "连对角线找到中心";
    ELLIPSE_STEP_2 = "Through it, the four sides' middles", "过中心找到四条边的中点";
    ELLIPSE_STEP_3 = "Two passes in the air", "在空中比划两遍";
    ELLIPSE_STEP_4 = "One stroke through the middles, from the shoulder",
        "从肩膀带动，一笔画过四个中点";
    ELLIPSE_SAYS = "In perspective a side's middle lies nearer its far end. Compare: the exact \
        ellipse in red, touching each side at its ringed middle, the one found in your lines \
        dashed in blue. Draw it again over yours in blue and compare: it reads your correction \
        alone.",
        "透视中边的中点更靠近远端。叠图对比：红色是准确的椭圆，与每条边相切于画圈的中点；蓝色虚线是从你的线条里找到的椭圆。用修正笔在上面再画一次，再对比：只看你修正的那一笔。";
}
