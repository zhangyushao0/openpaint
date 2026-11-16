//! Curved structure rotation's words (S154f): its name and its How card, after V大's method in our
//! own words (weeks 7 and 8): a curved object split into hemispheres, cylinders and cones, the
//! biggest part first and the rest in proportion; each joint's section, a circle in a square round
//! the axis, placed in the box; the outlines touching the ellipses; a spout a cone of its own, a
//! handle's profile drawn on a face and pulled through. In his terms: 曲面翻转, 半球, 圆柱, 圆锥,
//! 截面, 轴线, 接头, 透视线, 壶嘴, 壶把.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("curved-rotation", CURVED_EXERCISE, CURVED)];

const CURVED: How = How {
    heading: CURVED_HEADING,
    trains: Some(CURVED_TRAINS),
    steps: &[
        CURVED_STEP_1,
        CURVED_STEP_2,
        CURVED_STEP_3,
        CURVED_STEP_4,
        CURVED_STEP_5,
        COMPARE_STEP,
    ],
    says: &[CURVED_SAYS],
};

texts! { PRACTICE_CURVED_ROTATIONS;
    CURVED_EXERCISE = "Curved rotation", "曲面翻转";
    CURVED_HEADING = "A curved object from a new angle", "换个角度画曲面物体";
    CURVED_TRAINS = "A curved object split into hemispheres, cylinders and cones, drawn from an \
        angle you have not seen: the biggest part first, then each joint's section placed in the \
        box.",
        "把曲面物体拆成半球、圆柱和圆锥，从没看过的角度画出来：先定最大的一段，再把每个接头的截面放进盒子。";
    CURVED_STEP_1 = "See the parts in the picture and the box tight round them",
        "看清原图拆成哪几段，和紧贴它的盒子";
    CURVED_STEP_2 = "The same box at the new angle with Sketch", "用草稿笔在新角度画出同一个盒子";
    CURVED_STEP_3 = "A section's square at each joint along the axis",
        "沿着轴线，在每个接头画出截面的方框";
    CURVED_STEP_4 = "The ellipses in them, the outlines touching them",
        "在方框里画椭圆，再画和椭圆相切的轮廓";
    CURVED_STEP_5 = "The visible edges with Draw", "用勾线笔画出看得见的边";
    CURVED_SAYS = "The biggest part first, then each joint's place on the axis: a half, a third \
        or a quarter of the box. Cut the box across there; the section's square sits on the \
        cut's diagonals, its circle in it, and the outlines touch the ellipses. A handle's \
        profile goes on the side face, then is pulled through towards the vanishing point. \
        Redraw a wrong one once with Correct.",
        "先定最大的一段，再看每个接头在轴线上的位置：盒子的二分之一、三分之一还是四分之一。在那里把盒子横切开，截面的方框落在切面的对角线上，圆在方框里，轮廓和椭圆相切。壶把先在侧面上画出截面轮廓，再朝消失点拉过去。画错了用修正笔重画一次。";
}
