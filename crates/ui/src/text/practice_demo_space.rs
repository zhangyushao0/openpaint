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
        horizon.", "第 2 周 · 透视：同向的边交于视平线上的同一个消失点。";
    BOX_STEP_1 = "Run the given edges on to the horizon", "把给出的边延长到视平线";
    BOX_STEP_2 = "Back from the points to the far corners", "从消失点连回远处的角";
    BOX_STEP_3 = "The nine edges with Draw, each once, hidden ones too",
        "用勾线笔画九条边，每条一笔，背后的也画";
    BOX_SAYS = "Ghost each edge in the air, then draw it once. A wrong one: redraw it once with \
        Correct, never scratch over it. Compare lays the exact box over yours in red, its hidden \
        edges dashed; an edge of yours that misses its vanishing point is drawn on dashed red past \
        the point's ring, so you see by how much. Construction shows the lines from every corner \
        to the points.",
        "每条边先在空中比划，再一笔画出。画错了就用修正笔重画一次，不要来回描。叠图对比把准确的盒子用红色叠在你的盒子上，看不见的边画成虚线；你的边没对准消失点的，会用红色虚线延长，越过消失点的红圈，偏了多少一看便知。点“看画法”，可以看到每个角连向消失点的线。";
    FREE_TRAINS = "Week 2 · boxes of your own on a horizon, each turned its own way, every front \
        corner wider than 90°.", "第 2 周 · 视平线上自己画盒子，朝向各异，前角都大于 90°。";
    FREE_STEP_1 = "Two points on the horizon", "在视平线上定两个消失点";
    FREE_STEP_2 = "A front upright", "画最前面的竖边";
    FREE_STEP_3 = "Its edges towards the points", "从它两端连向消失点";
    FREE_STEP_4 = "The far uprights, then the back", "再画远处的竖边和后面的边";
    FREE_SAYS = "Draw three or four, each of other proportions; points too close distort a box. \
        Ghost each edge, draw it once, redraw a wrong one once with Correct. Compare rings in red \
        where each box's edges of a direction meet on the horizon; an edge that misses that point \
        is drawn on dashed red past it, a leaning vertical gets its true vertical, a front corner \
        under 90° an arc. Construction runs every edge on to the horizon.",
        "画三四个，长宽高比例各不相同；两个消失点太近，盒子会变形。每条边先在空中比划，再一笔画出，画错了用修正笔重画一次。叠图对比用红圈标出每个盒子同向的边在视平线上的交点；没对准这个点的边用红色虚线延长，越过它；歪了的竖边旁画出真正的竖直线，小于 90° 的前角画一段弧。点“看画法”，每条边都会延长到视平线。";
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
