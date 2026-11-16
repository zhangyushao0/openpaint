//! Contours' words (S156k): its name and its How card, after Drawabox's organic forms with contour
//! lines in our own words: a tube's sections as ellipses round its spine, the minor axis along the
//! spine, the degree opening where the tube turns towards you and closing where it runs across,
//! each drawn through twice round; at Steady only the half that faces you, and a small one round
//! each end. In Chinese terms: 截面线, 中轴, 短轴, 椭圆, 刻度, 轮廓, 画穿.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("contours", CONTOURS_EXERCISE, CONTOURS)];

const CONTOURS: How = How {
    heading: CONTOURS_HEADING,
    trains: Some(CONTOURS_TRAINS),
    steps: &[
        CONTOURS_STEP_1,
        CONTOURS_STEP_2,
        CONTOURS_STEP_3,
        COMPARE_STEP,
    ],
    says: &[CONTOURS_SAYS],
};

texts! { PRACTICE_CONTOURS;
    CONTOURS_EXERCISE = "Contours", "截面线";
    CONTOURS_HEADING = "A bending tube by its sections", "用截面线画弯曲的管子";
    CONTOURS_TRAINS = "How a bending tube turns in space, told by the ellipses round it.",
        "用一圈圈截面椭圆，表现弯曲的管子在空间里怎么转。";
    CONTOURS_STEP_1 = "Follow the spine: where the tube turns towards you, where away",
        "顺着中轴看：哪一段朝你转过来，哪一段转开";
    CONTOURS_STEP_2 = "At each tick, the minor axis along the spine with Sketch",
        "用草稿笔在每个刻度处画短轴，顺着中轴";
    CONTOURS_STEP_3 = "The ellipse round it with Draw, touching the outline on both sides",
        "用勾线笔绕短轴画椭圆，两边贴住轮廓";
    CONTOURS_SAYS = "An ellipse's minor axis always lies along the spine. Where the tube turns \
        towards you the ellipse opens rounder; where it runs across it closes flatter. Draw each \
        one through, twice round. At Steady draw only the half that faces you, and a small one \
        round each end. Redraw a wrong one once with Correct.",
        "椭圆的短轴总是顺着中轴。管子朝你转过来的地方，椭圆张得更圆；横着走的地方，椭圆收得更扁。每个椭圆都画穿，绕两圈。到了“稳”，只画朝向你的那半圈，两头各画一个小圈。画错了用修正笔重画一次。";
}
