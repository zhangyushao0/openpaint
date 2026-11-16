//! Form intersections' words (S156k, S158k): its name and a How card a level, after Drawabox's form
//! intersections and V大's weeks 7 and 8 in our own words: on two boxes the pierce points first,
//! each edge carried into the other form's face in that face's own perspective, then joined along
//! the faces; a cylinder's circle on a box's face drawn as the ellipse in its square; two round
//! forms cut by a few sections across both, where each section's lines cross a point of the line;
//! both forms drawn through. In his terms: 穿插, 穿插点, 交线, 截面, 消失点, 画穿.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 3] = [
    words("intersections", INTERSECTIONS_EXERCISE, INTERSECTIONS),
    words("intersections-cylinder", INTERSECTIONS_EXERCISE, CYLINDER),
    words("intersections-round", INTERSECTIONS_EXERCISE, ROUND),
];

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
const CYLINDER: How = How {
    heading: CYLINDER_HEADING,
    trains: Some(CYLINDER_TRAINS),
    steps: &[
        CYLINDER_STEP_1,
        CYLINDER_STEP_2,
        CYLINDER_STEP_3,
        COMPARE_STEP,
    ],
    says: &[CYLINDER_SAYS],
};
const ROUND: How = How {
    heading: ROUND_HEADING,
    trains: Some(ROUND_TRAINS),
    steps: &[ROUND_STEP_1, ROUND_STEP_2, ROUND_STEP_3, COMPARE_STEP],
    says: &[ROUND_SAYS],
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
        form's face, seen in that face's own perspective. Join them along the faces. Draw both \
        forms through; the part behind may be drawn lightly. Redraw a wrong one once with \
        Correct.",
        "先找穿插点：每条边穿过另一个形体的面的地方，要按那个面自己的透视去找。再沿着面把它们连起来。两个形体都要画穿，后面看不见的部分可以画轻一点。画错了用修正笔重画一次。";
    CYLINDER_HEADING = "A cylinder and a box", "圆柱和盒子穿插";
    CYLINDER_TRAINS = "Finding where a round form meets a flat one: on the box's face, the \
        cylinder's circle drawn as the ellipse in its square.",
        "找出圆的形体和平的形体的交线：在盒子的面上，把圆柱的圆画成正方形里的椭圆。";
    CYLINDER_STEP_1 = "On the face, the square the cylinder sits in, in the face's perspective, \
        and its diagonals", "在面上按这个面的透视画出圆柱所在的正方形和它的对角线";
    CYLINDER_STEP_2 = "Through their crossing, the middles of its sides",
        "过对角线的交点找出各边的中点";
    CYLINDER_STEP_3 = "The ellipse through the four middles with Draw",
        "用勾线笔画出经过四个中点的椭圆";
    CYLINDER_SAYS = "Where the cylinder meets the face, its circle lies in that face: draw the \
        square in the face's own perspective, then the ellipse touching each side at its middle. \
        Where a box goes into a cylinder's side, its faces cut the cylinder in straight lines \
        along it and in arcs of its sections across it. Draw both forms through; the part behind \
        may be drawn lightly. Redraw a wrong one once with Correct.",
        "圆柱和面相交的圆就在这个面上：按这个面自己的透视画出正方形，再画椭圆，在每条边的中点和边相切。盒子插进圆柱侧面时，盒子的面切开圆柱：顺着圆柱是直线，横着切过它是截面上的一段弧。两个形体都要画穿，后面看不见的部分可以画轻一点。画错了用修正笔重画一次。";
    ROUND_HEADING = "Two round forms", "两个圆的形体穿插";
    ROUND_TRAINS = "Finding the line where two round forms meet: sections cut across both, the \
        points where their lines cross.",
        "找出两个圆的形体的交线：同时切过两个形体的截面，截面上线条交叉的点。";
    ROUND_STEP_1 = "With Sketch, a few sections across both forms",
        "用草稿笔画几个同时切过两个形体的截面";
    ROUND_STEP_2 = "In each, where the two forms' lines cross: a point of the line",
        "每个截面上，两个形体的线交叉的地方就是交线上的一点";
    ROUND_STEP_3 = "Join the points with Draw, round as the forms",
        "用勾线笔顺着形体的圆把这些点连起来";
    ROUND_SAYS = "Cut each section along the forms' own directions, so its lines are easy to \
        find: the thinner cylinder's sides, carried to the other's surface. A ball sunk into a \
        cylinder is cut across the cylinder: in each section two circles, crossing on the line. \
        Draw what the eye sees; the part behind may be drawn lightly. Redraw a wrong one once \
        with Correct.",
        "顺着形体自己的方向切截面，线就容易找：细圆柱的两条边，一直延伸到粗圆柱的表面。球陷进圆柱时，横着圆柱切，每个截面上有两个圆，它们交叉的地方在交线上。画出看得见的部分，后面看不见的可以画轻一点。画错了用修正笔重画一次。";
}
