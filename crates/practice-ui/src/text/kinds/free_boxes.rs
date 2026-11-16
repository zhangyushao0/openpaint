//! Free boxes' words (week 2): their name and their How cards, after V大's
//! method in our own words (boxes of one's own on a horizon, each turned its own way, every front
//! corner wider than 90 degrees; looking down or up, the verticals meeting far below or above)
//! and Drawabox's (ghost a line before drawing it). In his terms: 自由盒子, 视平线, 消失点, 前角,
//! 俯视, 仰视, 三点透视, 叠图对比. A page that looks down and one that looks up (S142) are topics of
//! their own, each its How; a page with a path (S156l, Drawabox's organic perspective) too: boxes
//! sent back along it, near big and far small, each turned its own way, a nearer one hiding what
//! lies behind it; 有机透视, 近大远小.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 4] = [
    words("free-boxes", FREE_BOXES_EXERCISE, FREE_HOW),
    words("free-boxes-down", FREE_BOXES_EXERCISE, DOWN_HOW),
    words("free-boxes-up", FREE_BOXES_EXERCISE, UP_HOW),
    words("free-boxes-path", FREE_BOXES_EXERCISE, PATH_HOW),
];

const PATH_HOW: How = How {
    heading: ALONG_A_PATH,
    trains: Some(PATH_TRAINS),
    steps: &[PATH_STEP_1, PATH_STEP_2, PATH_STEP_3, COMPARE_STEP],
    says: &[PATH_SAYS],
};

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
        Ghost each edge, draw it once, redraw a wrong one once with Correct. To check a box, run \
        its edges on in pencil with Sketch: each direction's should meet at one point on the \
        horizon, its verticals stay vertical, its front corner wider than 90°.",
        "画三四个，长宽高比例各不相同；两个消失点太近，盒子会变形。每条边先在空中比划，再一笔画出，画错了用修正笔重画一次。想检查一个盒子，就用草稿笔把它的边延长：同向的边应该交于视平线上的同一点，竖边保持竖直，前角大于 90°。";
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
        To check a box, run its edges on in pencil with Sketch: each direction's should meet at \
        one point on the horizon, every upright at the third point, each front corner wider than \
        90°.",
        "画三四个，长宽高比例各不相同，所有竖边都朝向同一个第三消失点：画面给了淡线的，淡线就指向它；没给的，自己定在远处，不要落在画面里。每条边先在空中比划，再一笔画出，画错了用修正笔重画一次。想检查一个盒子，就用草稿笔把它的边延长：同向的边应该交于视平线上的同一点，所有竖边交于第三消失点，前角都大于 90°。";
    ALONG_A_PATH = "Boxes along a path", "沿路摆盒子";
    PATH_TRAINS = "Boxes sent back in depth along a path: near ones big, far ones small, each \
        turned its own way, a nearer one hiding what lies behind it.",
        "沿着一条路把盒子往远处摆：近大远小，朝向各异，近的挡住后面的。";
    PATH_STEP_1 = "The nearest box, big, where the path begins", "最近的盒子画大，放在路的近端";
    PATH_STEP_2 = "Farther along, a smaller one turned its own way",
        "沿路往远处，画小一点、换个朝向";
    PATH_STEP_3 = "Farther still, smaller again", "再往远，更小";
    PATH_SAYS = "Draw four or five along the path, each smaller the farther it stands, each \
        turned its own way, its edges towards points of its own on the horizon. Where a nearer \
        box stands in front of a farther one, leave out the farther one's lines behind it. Ghost \
        each edge, draw it once, redraw a wrong one once with Correct.",
        "沿着路画四五个，越远越小，朝向各异，每个盒子的边朝向它自己在视平线上的消失点。近的盒子挡在远的前面时，远处盒子被挡住的线不画。每条边先在空中比划，再一笔画出，画错了用修正笔重画一次。";
}
