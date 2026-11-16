//! Form intersections' words (S156k): its name and its How card, after Drawabox's form
//! intersections and V大's weeks 7 and 8 in our own words: the pierce points first, each edge
//! carried into the other form's face in that face's own perspective, then joined along the faces;
//! a round form cut by a few sections across both forms, where each section's lines cross a point
//! of the line; both forms drawn through. In his terms: 穿插, 穿插点, 交线, 截面, 消失点, 画穿.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words(
    "intersections",
    INTERSECTIONS_EXERCISE,
    INTERSECTIONS,
)];

const INTERSECTIONS: How = How {
    heading: INTERSECTIONS_HEADING,
    trains: Some(INTERSECTIONS_TRAINS),
    steps: &[
        INTERSECTIONS_STEP_1,
        INTERSECTIONS_STEP_2,
        INTERSECTIONS_STEP_3,
        COMPARE_STEP,
    ],
    says: &[INTERSECTIONS_SAYS],
};

texts! { PRACTICE_INTERSECTIONS;
    INTERSECTIONS_EXERCISE = "Form intersections", "穿插";
    INTERSECTIONS_HEADING = "Where two forms meet", "画两个形体相交的线";
    INTERSECTIONS_TRAINS = "Finding the line where one form passes into another: the pierce \
        points first, then the line between them.",
        "找出一个形体穿进另一个形体的交线：先找穿插点，再连成线。";
    INTERSECTIONS_STEP_1 = "Carry each edge into the other form's face, towards its vanishing \
        point", "把每条边延长到另一个形体的面上，朝着它的消失点";
    INTERSECTIONS_STEP_2 = "Where an edge meets a face: the pierce points",
        "边和面相交的地方，就是穿插点";
    INTERSECTIONS_STEP_3 = "Join them along the faces with Draw", "用勾线笔沿着面把这些点连起来";
    INTERSECTIONS_SAYS = "Find the pierce points first: where each edge passes through the other \
        form's face, seen in that face's own perspective. Join them along the faces. Where a \
        form is round, cut a few sections across both forms; where each section's lines cross is \
        a point of the line. Draw both forms through; the part behind may be drawn lightly. \
        Redraw a wrong one once with Correct.",
        "先找穿插点：每条边穿过另一个形体的面的地方，要按那个面自己的透视去找。再沿着面把它们连起来。遇到圆的形体，横着切几个截面，同时切过两个形体；每个截面上两条线交叉的地方，就是交线上的一点。两个形体都要画穿，后面看不见的部分可以画轻一点。画错了用修正笔重画一次。";
}
