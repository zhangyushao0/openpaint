//! Structure rotation's words (S113): its name and its How card, after V大's method in our own
//! words (weeks 5 to 8): the box tight round the object's main body, the object as a few solids in
//! it placed by halves and thirds, the same box at the new angle, then the visible edges; the angle
//! must change or nothing is trained; the big things right, the details felt. In his terms: 结构翻转,
//! 方块, 二等分, 三等分, 视平线, 消失点, 竖边.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("structure-rotation", ROTATION_EXERCISE, ROTATION)];

const ROTATION: How = How {
    heading: ROTATION_HEADING,
    trains: Some(ROTATION_TRAINS),
    steps: &[
        ROTATION_STEP_1,
        ROTATION_STEP_2,
        ROTATION_STEP_3,
        ROTATION_STEP_4,
        COMPARE_STEP,
    ],
    says: &[ROTATION_SAYS],
};

texts! { PRACTICE_ROTATIONS;
    ROTATION_EXERCISE = "Structure rotation", "结构翻转";
    ROTATION_HEADING = "An object from a new angle", "换个角度画单体";
    ROTATION_TRAINS = "An object seen as a few solids in a box, drawn from an angle you have not \
        seen: the box first, then each part by halves and thirds.",
        "把单体看成盒子里的几块几何体，从没看过的角度画出来：先定盒子，再用二等分、三等分定每一块。";
    ROTATION_STEP_1 = "See the box tight round the object in the picture",
        "看清原图里紧贴单体的盒子";
    ROTATION_STEP_2 = "The same box at the new angle with Sketch", "用草稿笔在新角度画出同一个盒子";
    ROTATION_STEP_3 = "Each part's points by halves and thirds", "用二等分、三等分定出每一块的点";
    ROTATION_STEP_4 = "The visible edges with Draw", "用勾线笔画出看得见的边";
    ROTATION_SAYS = "In the picture, find where each part sits in its box: a half, a third, a \
        quarter of an edge. Then find the same points in the new box, every edge towards its \
        vanishing point; the big shapes first, details by feel. Where only the near upright is \
        given, run the small box's edges on to find the vanishing points. Redraw a wrong one once \
        with Correct.",
        "先在原图里看清每一块在盒子里的位置：在边的二分之一、三分之一还是四分之一处。再在新盒子里找到同样的点，每条边都朝向它的消失点；先保证大的形体，细节可以凭感觉。只给了最前面的竖边时，把小方块的边延长，找到消失点。画错了用修正笔重画一次。";
}
