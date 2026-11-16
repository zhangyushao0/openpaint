//! Cast shadows' words (S153, 投影, 阴影透视): its name and How card, after the drawing of shadows
//! in perspective: sunlight as parallel rays, a lamp as rays from one point; each shadow corner
//! where the ray through a top corner meets the ground line through the corner under it. In its
//! terms: 光线, 地面线, 视平线, 光点, 投影的消失点, 灯, 落脚点.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("cast-shadows", SHADOWS_EXERCISE, SHADOWS)];

const SHADOWS: How = How {
    heading: SHADOWS_HEADING,
    trains: Some(SHADOWS_TRAINS),
    steps: &[
        SHADOWS_STEP_RAYS,
        SHADOWS_STEP_GROUND,
        SHADOWS_STEP_OUTLINE,
        COMPARE_STEP,
    ],
    says: &[SHADOWS_SAYS],
};

texts! { PRACTICE_CAST_SHADOWS;
    SHADOWS_EXERCISE = "Cast shadows", "投影";
    SHADOWS_HEADING = "A box's shadow on the ground", "画出盒子在地上的投影";
    SHADOWS_TRAINS = "Where a box's shadow falls in perspective: each corner of it where the ray \
        through a top corner meets the ground line through the corner under it.",
        "在透视里找出盒子的投影：经过顶角的光线，和经过它下方底角的地面线相交，交点就是投影的角。";
    SHADOWS_STEP_RAYS = "With Sketch, a ray through each top corner at the pin's slant",
        "用草稿笔经过每个顶角画光线，斜度和杆顶到影子尖一样";
    SHADOWS_STEP_GROUND = "A ground line through the corner under it, along the pin's shadow",
        "经过下方的底角画地面线，方向和小杆的影子一样";
    SHADOWS_STEP_OUTLINE = "Where they meet, the shadow's corners: its outline with Draw",
        "两线的交点是投影的角，用勾线笔画出投影的轮廓";
    SHADOWS_SAYS = "Where the sun's point is given, every ray runs from it and every ground line \
        from the point on the horizon under it; a lamp's rays run from the lamp, its ground lines \
        from its foot. Where a shadow meets another box's side it climbs it: from where the \
        ground line meets the side, straight up to the ray. Redraw a wrong one once with Correct.",
        "给了太阳的光点时，光线都从光点出发，地面线都从它正下方视平线上的点出发；灯光的光线从灯出发，地面线从灯的落脚点出发。投影碰到另一个盒子的侧面就爬上去：从地面线碰到侧面的地方竖直向上，直到光线。画错了用修正笔重画一次。";
}
