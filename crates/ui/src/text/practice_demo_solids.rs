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
    CYLINDER_SAYS = "Ghost each ellipse and side in the air, then draw it once, the far end's \
        hidden half too. Compare: the cylinder in red, its hidden part dashed; a middle your \
        ellipse misses by more than a pen's width ringed, a side off where it heads run on dashed \
        red. Construction shows the diagonals and middles; a correction counts in place of what \
        it redraws.",
        "每个椭圆、每条边先在空中比划，再一笔画出，远端看不见的一半也要画。叠图对比：红色是准确的圆柱，被挡住的部分是虚线；椭圆离某个中点超过一个笔宽，这个中点画红圈；边的方向偏了，用红色虚线延长。点“看画法”，可以看到对角线和中点；修正的线代替它重画的那部分。";
    CONE_TRAINS = "Week 3 · a cone found in its box: its base, then its point.",
        "第 3 周 · 在盒子里找圆锥：先底面，再找尖。";
    CONE_STEP_1 = "The base: diagonals, four middles, ellipse", "底面：对角线、四边中点、椭圆";
    CONE_STEP_2 = "The opposite face's diagonals: the point", "对面对角线找尖";
    CONE_STEP_3 = "Two lines from the point touching the ellipse", "从尖画两条线切于椭圆";
    CONE_SAYS = "The point is the opposite face's centre; the sides touch the base's ellipse, they \
        do not end at its middles. Ghost each line in the air, then draw it once. Compare: the \
        exact cone in red, the base's hidden part dashed; a middle your base misses by more than a \
        pen's width is ringed, a side that misses the point runs on dashed red past it. \
        Construction shows the diagonals, the middles and the point. Correct in blue and compare \
        again.",
        "尖在对面的中心；两边切于底面的椭圆，而不是连到中点。每条线先在空中比划，再一笔画出。叠图对比：红色是准确的圆锥，底面被挡住的部分是虚线；底面的椭圆离某个中点超过一个笔宽，这个中点就画上红圈；边没对准尖，会用红色虚线延长，越过尖。点“看画法”，可以看到对角线、中点和尖。用修正笔（蓝色）改，再对比。";
    SPHERE_TRAINS = "Week 3 · a sphere in its cube, V大's quick way: not exact, but near in most \
        views.", "第 3 周 · 立方体里的球体，V大 的简便画法：不精确，但多数角度足够接近。";
    SPHERE_STEP_1 = "Halve the box in height: the middle section", "盒子上下二等分，得中截面";
    SPHERE_STEP_2 = "Its diagonals and four middles", "中截面对角线、四边中点";
    SPHERE_STEP_3 = "The section's ellipse with Sketch", "画中截面的椭圆";
    SPHERE_STEP_4 = "A circle with Draw, touching the ellipse at its long axis's ends",
        "画圆，切于椭圆长轴两端";
    SPHERE_SAYS = "A sphere's outline is near a circle as wide as its middle section's ellipse. \
        Ghost the circle in the air, then draw it once. Compare: the exact outline in red; where \
        yours falls short of it or runs past by more than a pen's width, its end there is ringed. \
        Construction shows the box halved, the section's diagonals and its ellipse. Correct in \
        blue and compare again.",
        "球体的外轮廓接近一个圆，宽度等于中截面椭圆的长轴。先在空中比划，再一笔画出圆。叠图对比：红色是准确的外轮廓；你的圆在哪一边没碰到它或超出它一个笔宽以上，那一边的端点就画上红圈。点“看画法”，可以看到二等分的盒子、中截面的对角线和椭圆。用修正笔（蓝色）改，再对比。";
}
