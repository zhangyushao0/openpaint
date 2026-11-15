//! Free boxes' words (week 2): their name and their How cards, after V大's
//! method in our own words (boxes of one's own on a horizon, each turned its own way, every front
//! corner wider than 90 degrees; looking down or up, the verticals meeting far below or above)
//! and Drawabox's (ghost a line before drawing it). In his terms: 自由盒子, 视平线, 消失点, 前角,
//! 俯视, 仰视, 三点透视, 叠图对比. A page that looks down and one that looks up (S142) are topics of
//! their own, each its How.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 3] = [
    words("free-boxes", FREE_BOXES_EXERCISE, FREE_HOW),
    words("free-boxes-down", FREE_BOXES_EXERCISE, DOWN_HOW),
    words("free-boxes-up", FREE_BOXES_EXERCISE, UP_HOW),
];

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

const DOWN_HOW: How = How {
    heading: LOOKING_DOWN,
    trains: Some(DOWN_TRAINS),
    steps: &[
        DOWN_STEP_1,
        THIRD_STEP_2,
        FREE_STEP_3,
        THIRD_STEP_4,
        COMPARE_STEP,
    ],
    says: &[THIRD_SAYS],
};

const UP_HOW: How = How {
    heading: LOOKING_UP,
    trains: Some(UP_TRAINS),
    steps: &[
        UP_STEP_1,
        THIRD_STEP_2,
        FREE_STEP_3,
        THIRD_STEP_4,
        COMPARE_STEP,
    ],
    says: &[THIRD_SAYS],
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
    LOOKING_DOWN = "Free boxes, looking down", "自由盒子：俯视";
    LOOKING_UP = "Free boxes, looking up", "自由盒子：仰视";
    DOWN_TRAINS = "Looking down from a high horizon: every box's uprights meet at one point far \
        below.", "从高视平线往下看：所有盒子的竖边交于下方远处的同一点。";
    UP_TRAINS = "Looking up from a low horizon: every box's uprights meet at one point far \
        above.", "从低视平线往上看：所有盒子的竖边交于上方远处的同一点。";
    DOWN_STEP_1 = "Two points on the horizon, a third far below",
        "视平线上定两个消失点，第三个在下方远处";
    UP_STEP_1 = "Two points on the horizon, a third far above",
        "视平线上定两个消失点，第三个在上方远处";
    THIRD_STEP_2 = "A front upright towards the third point", "画最前面的竖边，朝向第三个消失点";
    THIRD_STEP_4 = "The far uprights towards the third point, then the back",
        "远处的竖边也朝向第三个点，再画后面的边";
    THIRD_SAYS = "Draw three or four, each of other proportions, every upright towards the one \
        third point: where faint lines are given they lead to it; without them choose it far off, \
        never on the page. Ghost each edge, draw it once, redraw a wrong one once with Correct. \
        Compare rings in red where each box's edges of a direction meet on the horizon and the \
        third point (where your uprights meet when none is given); an upright that misses it gets \
        its true upright, a front corner under 90° an arc. Construction runs every edge on to its \
        point.",
        "画三四个，长宽高比例各不相同，所有竖边都朝向同一个第三消失点：画面给了淡线的，淡线就指向它；没给的，自己定在远处，不要落在画面里。每条边先在空中比划，再一笔画出，画错了用修正笔重画一次。叠图对比用红圈标出每个盒子同向的边在视平线上的交点和第三消失点（没给的，就是你的竖边相交的地方）；没对准它的竖边旁画出真正的竖边方向，小于 90° 的前角画一段弧。点“看画法”，每条边都会延长到它的消失点。";
}
