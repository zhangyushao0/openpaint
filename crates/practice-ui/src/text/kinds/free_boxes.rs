//! Free boxes' words (week 2): their name and their How card, after V大's
//! method in our own words (boxes of one's own on a horizon, each turned its own way, every front
//! corner wider than 90 degrees) and Drawabox's (ghost a line before drawing it). In his terms:
//! 自由盒子, 视平线, 消失点, 前角, 叠图对比.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("free-boxes", FREE_BOXES_EXERCISE, FREE_HOW)];

const FREE_HOW: How = How {
    heading: FREE_BOXES_EXERCISE,
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

texts! { PRACTICE_FREE_BOXES;
    FREE_BOXES_EXERCISE = "Free boxes", "自由盒子";
    FREE_TRAINS = "Boxes of your own on a horizon, each turned its own way, every front \
        corner wider than 90°.", "视平线上自己画盒子，朝向各异，前角都大于 90°。";
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
}
