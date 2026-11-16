//! Ghosted lines' words (S156, 鬼影线, after Drawabox's lesson 1): its three topics' names and How
//! cards, superimposed lines (叠线), ghosted lines between two points (鬼影线) and ghosted planes
//! (鬼影面). In its terms: 定点, 比划, 一笔画出, 用肩膀带动, 起点, 终点, 对边, 中心.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 3] = [
    words("superimposed-lines", SUPERIMPOSED_EXERCISE, SUPERIMPOSED),
    words("ghosted-pairs", GHOSTED_EXERCISE, GHOSTED),
    words("ghosted-planes", PLANES_EXERCISE, PLANES),
];

const SUPERIMPOSED: How = How {
    heading: SUPERIMPOSED_HEADING,
    trains: Some(SUPERIMPOSED_TRAINS),
    steps: &[
        SUPERIMPOSED_STEP_GHOST,
        SUPERIMPOSED_STEP_EIGHT,
        COMPARE_STEP,
    ],
    says: &[SUPERIMPOSED_SAYS],
};
const GHOSTED: How = How {
    heading: GHOSTED_HEADING,
    trains: Some(GHOSTED_TRAINS),
    steps: &[
        GHOSTED_STEP_PLAN,
        GHOSTED_STEP_GHOST,
        GHOSTED_STEP_STROKE,
        COMPARE_STEP,
    ],
    says: &[GHOSTED_SAYS],
};
const PLANES: How = How {
    heading: PLANES_HEADING,
    trains: Some(PLANES_TRAINS),
    steps: &[
        PLANES_STEP_EDGES,
        PLANES_STEP_DIAGONALS,
        PLANES_STEP_MIDDLES,
        COMPARE_STEP,
    ],
    says: &[PLANES_SAYS],
};

texts! { PRACTICE_GHOSTED_LINES;
    SUPERIMPOSED_EXERCISE = "Superimposed lines", "叠线";
    GHOSTED_EXERCISE = "Ghosted lines", "鬼影线";
    PLANES_EXERCISE = "Ghosted planes", "鬼影面";
    SUPERIMPOSED_HEADING = "Strokes over a line", "沿一条线叠画";
    SUPERIMPOSED_TRAINS = "A confident stroke that starts where it is aimed: eight strokes over one \
        line, each from its dot.",
        "练果断的一笔，起笔落在要落的地方：沿同一条线画八笔，每笔都从起点出发。";
    SUPERIMPOSED_STEP_GHOST = "Ghost the motion along the line from the shoulder, a few times",
        "笔悬在线的上方，用肩膀带动，比划几遍";
    SUPERIMPOSED_STEP_EIGHT = "Pen down on the dot: eight strokes over the line, each in one go",
        "笔尖落在起点上，沿线一笔画完，叠画八笔";
    SUPERIMPOSED_SAYS = "Draw from the shoulder at a steady speed, never slowly. The far end may \
        fray a little; the start should not wander. Don't go back over a stroke: draw the next.",
        "用肩膀带动手臂，匀速，不要慢。远端稍微散开没关系，起笔不能偏。画坏了不要描，直接画下一笔。";
    GHOSTED_HEADING = "A line between two points", "两点之间画一条直线";
    GHOSTED_TRAINS = "Plan, ghost, then draw: a straight line from one point to another in one \
        sure stroke.",
        "先定点，再比划，后下笔：从一个点到另一个点，一笔画出直线。";
    GHOSTED_STEP_PLAN = "Plan: the point to start from and the one to end at",
        "定点：看准起点和终点";
    GHOSTED_STEP_GHOST = "Ghost the motion from the shoulder several times, the pen just above",
        "笔悬在纸面上方，用肩膀带动比划几遍";
    GHOSTED_STEP_STROKE = "Then one confident stroke with Draw", "然后用勾线笔果断地一笔画出";
    GHOSTED_SAYS = "Start from whichever point lets your arm move easily. Look at the end, not at \
        the pen's tip. Speed keeps the line straight; slowing down makes it wobble.",
        "从顺手的那一端起笔。眼睛看终点，不要盯着笔尖。快一点线才直，慢下来反而会抖。";
    PLANES_HEADING = "A plane through its four corners", "连出透视平面";
    PLANES_TRAINS = "Ghosted lines on a plane in perspective: its edges, its diagonals, and \
        through its centre the two lines that halve it.",
        "在透视平面上练鬼影线：四条边、两条对角线，再过中心画出把它平分的两条线。";
    PLANES_STEP_EDGES = "Each edge from corner to corner, ghosted first", "每条边从角点画到角点，先比划再下笔";
    PLANES_STEP_DIAGONALS = "The diagonals: where they cross is the centre", "画两条对角线：交点就是中心";
    PLANES_STEP_MIDDLES = "Through the centre, a line toward where each pair of opposite edges meet",
        "过中心各画一条线，朝向一组对边延长后的交点";
    PLANES_SAYS = "Opposite edges meet far off the frame, or run parallel: the line through the \
        centre heads there too and ends on the other two edges. Every line is one stroke, planned \
        and ghosted first.",
        "对边延长会在画框外很远处相交，或者互相平行：过中心的线也朝那里去，两端落在另外两条边上。每条线都是一笔，先定点，再比划。";
}
