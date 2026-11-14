//! The estimates' words (stage 3, V大's 二维抽象网格): a point's, a proportion's and an angle's names,
//! How cards, after his method in our own words (points by the frame and long
//! lines, never a grid drawn; proportions by the half and the thirds; angles by an imagined cross
//! or the line run on to the frame's edge). In his terms: 定点, 按比例找点, 叠图对比.

use openpaint_practice::Estimate;

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 3] = [
    words("point", POINT_EXERCISE, estimate(Estimate::Point)),
    words(
        "proportion",
        PROPORTION_EXERCISE,
        estimate(Estimate::Proportion),
    ),
    words("angle", ANGLE_EXERCISE, estimate(Estimate::Angle)),
];

/// An estimate's How card.
const fn estimate(estimate: Estimate) -> How {
    match estimate {
        Estimate::Point => How {
            heading: PLACE_A_POINT,
            trains: Some(POINT_TRAINS),
            steps: &[POINT_STEP_1, POINT_STEP_2, POINT_STEP_3, COMPARE_STEP],
            says: &[POINT_SAYS],
        },
        Estimate::Proportion => How {
            heading: DIVIDE_ALIKE,
            trains: Some(PROPORTION_TRAINS),
            steps: &[
                PROPORTION_STEP_1,
                PROPORTION_STEP_2,
                PROPORTION_STEP_3,
                COMPARE_STEP,
            ],
            says: &[PROPORTION_SAYS],
        },
        Estimate::Angle => How {
            heading: COPY_AN_ANGLE,
            trains: Some(ANGLE_TRAINS),
            steps: &[ANGLE_STEP_1, ANGLE_STEP_2, ANGLE_STEP_3, COMPARE_STEP],
            says: &[ANGLE_SAYS],
        },
    }
}

texts! { PRACTICE_ESTIMATES;
    POINT_EXERCISE = "Point", "定点";
    PROPORTION_EXERCISE = "Proportion", "比例";
    ANGLE_EXERCISE = "Angle", "角度";
    PLACE_A_POINT = "Place a point", "定点";
    DIVIDE_ALIKE = "Divide alike", "按比例找点";
    COPY_AN_ANGLE = "Copy an angle", "临摹角度";
    POINT_TRAINS = "A point placed by the frame alone, never a grid.",
        "只凭方框定点，不画网格。";
    POINT_STEP_1 = "Run it across and down to the edges", "把点横向、纵向引到边上";
    POINT_STEP_2 = "Against the halves and thirds", "和 1/2、1/3 处比一比";
    POINT_STEP_3 = "The same place, marked with Draw", "用勾线笔在相同位置标出";
    POINT_SAYS = "Look again and again before you mark; Sketch for your own guides. Compare: the exact point in red over your frame, ringed when your mark lies more \
        than a pen's width off it. Construction runs it across and down to the edges, the halves \
        and thirds ticked. Correct in blue and compare again.",
        "反复比较后再下笔；草稿笔画你自己的辅助线。叠图对比：红色是准确的点；你的标记偏了一个笔宽以上，这个点就画上红圈。点“看画法”，会把点横向、纵向引到边上，并标出 1/2 和 1/3。用修正笔（蓝色）改，再对比。";
    PROPORTION_TRAINS = "Dividing alike: shares compared, not lengths.",
        "按比例找点：比的是比例，不是长度。";
    PROPORTION_STEP_1 = "Halves and thirds on the given segment", "在给出的线段上找 1/2 和 1/3";
    PROPORTION_STEP_2 = "And on yours, from its ringed end", "在你的线段上也找出来，从带圈的一端量起";
    PROPORTION_STEP_3 = "The same share: a tick with Draw", "用勾线笔在相同比例处画一道短线";
    PROPORTION_SAYS = "The ringed ends answer each other. Compare: the exact \
        place in red across your segment, ringed when your mark lies more than a pen's width off \
        it along the segment. Construction ticks both segments' halves and thirds. Correct in blue \
        and compare again.",
        "两条线段带圈的端点相对应。叠图对比：红色短线是准确的位置；你的标记沿线段偏了一个笔宽以上，这里就画上红圈。点“看画法”，会在两条线段上标出 1/2 和 1/3。用修正笔（蓝色）改，再对比。";
    ANGLE_TRAINS = "An angle seen against level and upright.",
        "对照水平和竖直看角度。";
    ANGLE_STEP_1 = "A level and upright cross through it", "过线条想象一个水平竖直的十字";
    ANGLE_STEP_2 = "Run it on to the frame's edges", "把线延长到方框边上";
    ANGLE_STEP_3 = "Through the dot in one stroke with Draw", "用勾线笔过点一笔画出";
    ANGLE_SAYS = "Compare: the exact line in red through the dot. Yours, if \
        it turns from it, runs on dashed red to the frame's edges, so you see by how much; if it \
        misses the dot, the dot is ringed. Construction runs both lines on to the frames' edges, \
        an upright cross through each. Correct in blue and compare again.",
        "叠图对比：红色是过点的准确线条。你的线方向偏了，会用红色虚线延长到方框边上，偏了多少一看便知；没有经过那个点，这个点就画上红圈。点“看画法”，会把两条线都延长到方框边上，并画出穿过线条的十字。用修正笔（蓝色）改，再对比。";
}
