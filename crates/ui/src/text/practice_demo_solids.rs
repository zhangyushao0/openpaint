//! Week 3's solids' How cards (`practice_demo.rs`, S93), after V大's method in our own words: never
//! a cylinder straight off, the box first; an end's or a base's diagonals for its centre, lines
//! through it towards the vanishing points for the four middles, a smooth ellipse through them
//! touching the edges there; a cone's point at the opposite face's centre; a sphere by its middle
//! section's ellipse and a circle through the ends of its long axis, near in most views. In his
//! terms: 圆柱, 圆锥, 球体, 对角线, 中心, 四边中点, 相切, 椭圆, 长轴, 中截面, 消失点.

use super::practice_demo::{COMPARE_STEP, How};

pub(crate) const CYLINDER: How = How {
    heading: super::CYLINDER_IN_ITS_BOX,
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
pub(crate) const CONE: How = How {
    heading: super::CONE_IN_ITS_BOX,
    trains: Some(CONE_TRAINS),
    steps: &[CONE_STEP_1, CONE_STEP_2, CONE_STEP_3, COMPARE_STEP],
    says: &[CONE_SAYS],
};
pub(crate) const SPHERE: How = How {
    heading: super::SPHERE_IN_ITS_CUBE,
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

texts! { PRACTICE_DEMO_SOLIDS;
    CYLINDER_TRAINS = "Week 3 · a cylinder found in its box: the box first, never the cylinder \
        straight off.", "第 3 周 · 在盒子里找圆柱：先画盒子，不要直接画圆柱。";
    CYLINDER_STEP_1 = "An end's diagonals: its centre", "对角线找端面中心";
    CYLINDER_STEP_2 = "Through it towards the vanishing points: the four middles",
        "过中心连向消失点，得四边中点";
    CYLINDER_STEP_3 = "The ellipse through the middles with Draw, touching the edges",
        "过四个中点画椭圆，切于四边";
    CYLINDER_STEP_4 = "The other end alike", "另一端同样";
    CYLINDER_STEP_5 = "Two straight lines touching both ellipses", "两条直线同时切两个椭圆";
    CYLINDER_SAYS = "Construct with Sketch; the ellipses and sides with Draw, the far end's hidden \
        half too. Compare: the exact cylinder in red, its hidden part dashed; what was read of your \
        lines dashed in blue (an ellipse drawn far enough round fitted whole). Correct in blue and \
        compare again: a correction counts in place of what it redraws.",
        "草稿笔作图；画图笔画椭圆和两边，远端看不见的一半也要画。叠图对比：红色是准确的圆柱，被挡住的部分是虚线；蓝色虚线是从你的线条里读出的部分（画够一圈的椭圆拟合成完整的椭圆）。用修正笔（蓝色）改，再对比：修正的线代替它重画的那部分。";
    CONE_TRAINS = "Week 3 · a cone found in its box: its base, then its point.",
        "第 3 周 · 在盒子里找圆锥：先底面，再找尖。";
    CONE_STEP_1 = "The base: diagonals, four middles, ellipse", "底面：对角线、四边中点、椭圆";
    CONE_STEP_2 = "The opposite face's diagonals: the point", "对面对角线找尖";
    CONE_STEP_3 = "Two lines from the point touching the ellipse", "从尖画两条线切于椭圆";
    CONE_SAYS = "The point is the opposite face's centre; the sides touch the base's ellipse, they \
        do not end at its middles. Compare: the exact cone in red, the base's hidden part dashed; \
        what was read of your lines dashed in blue. Correct in blue and compare again.",
        "尖在对面的中心；两边切于底面的椭圆，而不是连到中点。叠图对比：红色是准确的圆锥，底面被挡住的部分是虚线；蓝色虚线是从你的线条里读出的部分。用修正笔（蓝色）改，再对比。";
    SPHERE_TRAINS = "Week 3 · a sphere in its cube, V大's quick way: not exact, but near in most \
        views.", "第 3 周 · 立方体里的球体，V大 的简便画法：不精确，但多数角度足够接近。";
    SPHERE_STEP_1 = "Halve the box in height: the middle section", "盒子上下二等分，得中截面";
    SPHERE_STEP_2 = "Its diagonals and four middles", "中截面对角线、四边中点";
    SPHERE_STEP_3 = "The section's ellipse with Sketch", "画中截面的椭圆";
    SPHERE_STEP_4 = "A circle with Draw, touching the ellipse at its long axis's ends",
        "画圆，切于椭圆长轴两端";
    SPHERE_SAYS = "A sphere's outline is near a circle as wide as its middle section's ellipse. \
        Compare: the exact outline in red; what was read of your circle dashed in blue. Correct in \
        blue and compare again.",
        "球体的外轮廓接近一个圆，宽度等于中截面椭圆的长轴。叠图对比：红色是准确的外轮廓；蓝色虚线是从你的圆里读出的部分。用修正笔（蓝色）改，再对比。";
}
