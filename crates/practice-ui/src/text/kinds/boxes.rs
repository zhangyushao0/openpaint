//! A box's words (week 2): its name and its How card, after V大's method in our
//! own words (edges running the same way meet at one vanishing point, those parallel to the ground
//! on the horizon; the front angle wider than 90 degrees; the hidden edges drawn too) and
//! Drawabox's (ghost a line before drawing it). In his terms: 透视盒子, 视平线, 消失点, 前角, 叠图对比.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("box", BOX_EXERCISE, BOX_HOW)];

const BOX_HOW: How = How {
    heading: COMPLETE_THE_BOX,
    trains: Some(BOX_TRAINS),
    steps: &[BOX_STEP_1, BOX_STEP_2, BOX_STEP_3, COMPARE_STEP],
    says: &[BOX_SAYS],
};

texts! { PRACTICE_BOXES;
    BOX_EXERCISE = "Box", "透视盒子";
    COMPLETE_THE_BOX = "Complete the box", "补全盒子";
    BOX_TRAINS = "Perspective: edges running one way meet at one vanishing point on the \
        horizon.", "透视：同向的边交于视平线上的同一个消失点。";
    BOX_STEP_1 = "Run the given edges on to the horizon", "把给出的边延长到视平线";
    BOX_STEP_2 = "Back from the points to the far corners", "从消失点连回远处的角";
    BOX_STEP_3 = "The nine edges with Draw, each once, hidden ones too",
        "用勾线笔画九条边，每条一笔，背后的也画";
    BOX_SAYS = "Ghost each edge in the air, then draw it once. A wrong one: redraw it once with \
        Correct, never scratch over it. To check an edge, run it on in pencil with Sketch: it \
        should reach its vanishing point.",
        "每条边先在空中比划，再一笔画出。画错了就用修正笔重画一次，不要来回描。想检查一条边，就用草稿笔把它延长：它应该交到自己的消失点上。";
}
