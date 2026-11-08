//! A box's words (week 2): its name, its summary's line and its How card, after V大's method in our
//! own words (edges running the same way meet at one vanishing point, those parallel to the ground
//! on the horizon; the front angle wider than 90 degrees; the hidden edges drawn too) and
//! Drawabox's (ghost a line before drawing it). In his terms: 透视盒子, 视平线, 消失点, 前角, 叠图对比.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("box", (BOX_EXERCISE, BOXES_COUNTED), BOX_HOW)];

const BOX_HOW: How = How {
    heading: COMPLETE_THE_BOX,
    trains: Some(BOX_TRAINS),
    steps: &[BOX_STEP_1, BOX_STEP_2, BOX_STEP_3, COMPARE_STEP],
    says: &[BOX_SAYS],
};

texts! { PRACTICE_BOXES;
    BOX_EXERCISE = "Box", "透视盒子";
    BOXES_COUNTED = "Boxes", "透视盒子";
    COMPLETE_THE_BOX = "Complete the box", "补全盒子";
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
}
