//! The solids' words (week 3, S93): a cylinder's, a cone's and a sphere's names
//! and How cards, after V大's method in our own words: never a cylinder straight off, the box first; an end's or a base's diagonals for its centre, lines
//! through it towards the vanishing points for the four middles, a smooth ellipse through them
//! touching the edges there; a cone's point at the opposite face's centre; a sphere by its middle
//! section's ellipse and a circle through the ends of its long axis, near in most views. In his
//! terms: 圆柱, 圆锥, 球体, 对角线, 中心, 四边中点, 相切, 椭圆, 长轴, 中截面, 消失点.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 3] = [
    words("cylinder", CYLINDER_EXERCISE, CYLINDER),
    words("cone", CONE_EXERCISE, CONE),
    words("sphere", SPHERE_EXERCISE, SPHERE),
];

const CYLINDER: How = How {
    heading: CYLINDER_IN_ITS_BOX,
    trains: Some(CYLINDER_TRAINS),
    steps: &[
        CYLINDER_STEP_1,
        CYLINDER_STEP_2,
        CYLINDER_STEP_3,
        CYLINDER_STEP_4,
        CYLINDER_STEP_5,
        COMPARE_STEP,
    ],
    says: &[CYLINDER_SAYS],
};
const CONE: How = How {
    heading: CONE_IN_ITS_BOX,
    trains: Some(CONE_TRAINS),
    steps: &[CONE_STEP_1, CONE_STEP_2, CONE_STEP_3, COMPARE_STEP],
    says: &[CONE_SAYS],
};
const SPHERE: How = How {
    heading: SPHERE_IN_ITS_CUBE,
    trains: Some(SPHERE_TRAINS),
    steps: &[
        SPHERE_STEP_1,
        SPHERE_STEP_2,
        SPHERE_STEP_3,
        SPHERE_STEP_4,
        COMPARE_STEP,
    ],
    says: &[SPHERE_SAYS],
};

texts! { PRACTICE_SOLIDS_HOW;
    CYLINDER_EXERCISE = "Cylinder", "圆柱";
    CONE_EXERCISE = "Cone", "圆锥";
    SPHERE_EXERCISE = "Sphere", "球体";
    CYLINDER_IN_ITS_BOX = "A cylinder in its box", "在盒子里画圆柱";
    CONE_IN_ITS_BOX = "A cone in its box", "在盒子里画圆锥";
    SPHERE_IN_ITS_CUBE = "A sphere in its cube", "在立方体里画球体";
    CYLINDER_TRAINS = "A cylinder found in its box: the box first, never the cylinder \
        straight off.", "在盒子里找圆柱：先画盒子，不要直接画圆柱。";
    CYLINDER_STEP_1 = "An end's diagonals: its centre", "用对角线找出端面中心";
    CYLINDER_STEP_2 = "Through it towards the vanishing points: the four middles",
        "过中心连向消失点，找出四边中点";
    CYLINDER_STEP_3 = "The ellipse through the middles with Draw, touching the edges",
        "用勾线笔过四个中点画椭圆，切于四边";
    CYLINDER_STEP_4 = "The other end alike", "另一端同样画";
    CYLINDER_STEP_5 = "Two straight lines touching both ellipses", "画两条直线，同时切于两个椭圆";
    CYLINDER_SAYS = "Ghost each ellipse and side in the air, then draw it once, the far end's \
        hidden half too. Redraw a wrong one once with Correct.",
        "每个椭圆、每条边先在空中比划，再一笔画出，远端看不见的一半也要画。画错了用修正笔重画一次。";
    CONE_TRAINS = "A cone found in its box: its base, then its point.",
        "在盒子里找圆锥：先画底面，再找锥尖。";
    CONE_STEP_1 = "The base: diagonals, four middles, ellipse", "先画底面：对角线、四边中点、椭圆";
    CONE_STEP_2 = "The opposite face's diagonals: the point", "用对面的对角线找出锥尖";
    CONE_STEP_3 = "Two lines from the point touching the ellipse", "从锥尖画两条线，切于椭圆";
    CONE_SAYS = "The point is the opposite face's centre; the sides touch the base's ellipse, they \
        do not end at its middles. Ghost each line in the air, then draw it once. Redraw a wrong \
        one once with Correct.",
        "锥尖在对面的中心；两条边切于底面的椭圆，而不是连到中点。每条线先在空中比划，再一笔画出。画错了用修正笔重画一次。";
    SPHERE_TRAINS = "A sphere in its cube, V大's quick way: not exact, but near in most \
        views.", "立方体里的球体，V大的简便画法：不精确，但多数角度足够接近。";
    SPHERE_STEP_1 = "Halve the box in height: the middle section", "把盒子上下二等分，找出中截面";
    SPHERE_STEP_2 = "Its diagonals and four middles", "画中截面的对角线和四边中点";
    SPHERE_STEP_3 = "The section's ellipse with Sketch", "用草稿笔画中截面的椭圆";
    SPHERE_STEP_4 = "A circle with Draw, touching the ellipse at its long axis's ends",
        "用勾线笔画圆，切于椭圆长轴两端";
    SPHERE_SAYS = "A sphere's outline is near a circle as wide as its middle section's ellipse. \
        Ghost the circle in the air, then draw it once. Redraw a wrong one once with Correct.",
        "球体的外轮廓接近一个圆，宽度等于中截面椭圆的长轴。先在空中比划，再一笔画出圆。画错了用修正笔重画一次。";
}
