//! Stage 3's How cards (`practice_demo.rs`), after V大's method in our own words: the estimates on
//! the flat (二维抽象网格: points by the frame and long lines, never a grid drawn; proportions by
//! the half and the thirds; angles by an imagined cross or the line run on to the frame's edge)
//! and his five tricks for points in space (点定位: diagonals for the centre, the line to the
//! vanishing point halving, equal parts by the near edge shown true, equal steps by the far edge's
//! middle, a mirror by the rectangle to the middle; every point needs a reason). In his terms:
//! 点定位, 二等分, 三等分, 等距扩展, 对称点, 消失点, 对角线, 叠图对比.

use openpaint_practice::{Estimate, Trick};

use super::Text;
use super::practice_demo::{COMPARE_STEP, How};

/// An estimate's How card.
pub(crate) const fn estimate(estimate: Estimate) -> How {
    match estimate {
        Estimate::Point => How {
            heading: super::PLACE_A_POINT,
            trains: Some(POINT_TRAINS),
            steps: &[POINT_STEP_1, POINT_STEP_2, POINT_STEP_3, COMPARE_STEP],
            says: &[POINT_SAYS],
        },
        Estimate::Proportion => How {
            heading: super::DIVIDE_ALIKE,
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
            heading: super::COPY_AN_ANGLE,
            trains: Some(ANGLE_TRAINS),
            steps: &[ANGLE_STEP_1, ANGLE_STEP_2, ANGLE_STEP_3, COMPARE_STEP],
            says: &[ANGLE_SAYS],
        },
    }
}

/// A trick's How card.
pub(crate) const fn trick(trick: Trick) -> How {
    let (heading, trains, steps, says): (Text, Text, &'static [Text], &'static [Text]) = match trick
    {
        Trick::Centre => (
            super::FIND_THE_CENTRE,
            CENTRE_TRAINS,
            &[CENTRE_STEP_1, CENTRE_STEP_2, COMPARE_STEP],
            &[CENTRE_SAYS],
        ),
        Trick::Halves => (
            super::HALVE_THE_FACE,
            HALVES_TRAINS,
            &[HALVES_STEP_1, HALVES_STEP_2, HALVES_STEP_3, COMPARE_STEP],
            &[HALVES_SAYS],
        ),
        Trick::Thirds => (
            super::THIRDS_IN_DEPTH,
            THIRDS_TRAINS,
            &[
                THIRDS_STEP_1,
                THIRDS_STEP_2,
                THIRDS_STEP_3,
                THIRDS_STEP_4,
                COMPARE_STEP,
            ],
            &[THIRDS_SAYS],
        ),
        Trick::Step => (
            super::THE_NEXT_FACE,
            STEP_TRAINS,
            &[
                STEP_STEP_1,
                STEP_STEP_2,
                STEP_STEP_3,
                STEP_STEP_4,
                COMPARE_STEP,
            ],
            &[STEP_SAYS],
        ),
        Trick::Mirror => (
            super::MIRROR_A_POINT,
            MIRROR_TRAINS,
            &[
                MIRROR_STEP_1,
                MIRROR_STEP_2,
                MIRROR_STEP_3,
                MIRROR_STEP_4,
                MIRROR_STEP_5,
                COMPARE_STEP,
            ],
            &[MIRROR_SAYS],
        ),
    };
    How {
        heading,
        trains: Some(trains),
        steps,
        says,
    }
}

texts! { PRACTICE_DEMO_POINTS;
    POINT_TRAINS = "Week 1 · a point placed by the frame alone, never a grid.",
        "第 1 周 · 只凭方框定点，不画网格。";
    POINT_STEP_1 = "Run it across and down to the edges", "把点横向、纵向引到边上";
    POINT_STEP_2 = "Against the halves and thirds", "和 1/2、1/3 处比一比";
    POINT_STEP_3 = "The same place, marked with Draw", "用勾线笔在相同位置标出";
    POINT_SAYS = "Look again and again before you mark; the last mark counts, Sketch is yours for \
        guides. Compare: the exact point in red over your frame, ringed when your mark lies more \
        than a pen's width off it. Construction runs it across and down to the edges, the halves \
        and thirds ticked. Correct in blue and compare again.",
        "反复比较后再下笔；以最后一个标记为准，草稿笔可以画你自己的辅助线。叠图对比：红色是准确的点；你的标记偏了一个笔宽以上，这个点就画上红圈。点“看画法”，会把点横向、纵向引到边上，并标出 1/2 和 1/3。用修正笔（蓝色）改，再对比。";
    PROPORTION_TRAINS = "Week 1 · dividing alike: shares compared, not lengths.",
        "第 1 周 · 按比例找点：比的是比例，不是长度。";
    PROPORTION_STEP_1 = "Halves and thirds on the given segment", "在给出的线段上找 1/2 和 1/3";
    PROPORTION_STEP_2 = "And on yours, from its ringed end", "在你的线段上也找出来，从带圈的一端量起";
    PROPORTION_STEP_3 = "The same share: a tick with Draw", "用勾线笔在相同比例处画一道短线";
    PROPORTION_SAYS = "The ringed ends answer each other; the last mark counts. Compare: the exact \
        place in red across your segment, ringed when your mark lies more than a pen's width off \
        it along the segment. Construction ticks both segments' halves and thirds. Correct in blue \
        and compare again.",
        "两条线段带圈的端点相对应；以最后一个标记为准。叠图对比：红色短线是准确的位置；你的标记沿线段偏了一个笔宽以上，这里就画上红圈。点“看画法”，会在两条线段上标出 1/2 和 1/3。用修正笔（蓝色）改，再对比。";
    ANGLE_TRAINS = "Week 1 · an angle seen against level and upright.",
        "第 1 周 · 对照水平和竖直看角度。";
    ANGLE_STEP_1 = "A level and upright cross through it", "过线条想象一个水平竖直的十字";
    ANGLE_STEP_2 = "Run it on to the frame's edges", "把线延长到方框边上";
    ANGLE_STEP_3 = "Through the dot in one stroke with Draw", "用勾线笔过点一笔画出";
    ANGLE_SAYS = "The last line counts. Compare: the exact line in red through the dot. Yours, if \
        it turns from it, runs on dashed red to the frame's edges, so you see by how much; if it \
        misses the dot, the dot is ringed. Construction runs both lines on to the frames' edges, \
        an upright cross through each. Correct in blue and compare again.",
        "以最后一条线为准。叠图对比：红色是过点的准确线条。你的线方向偏了，会用红色虚线延长到方框边上，偏了多少一看便知；没有经过那个点，这个点就画上红圈。点“看画法”，会把两条线都延长到方框边上，并画出穿过线条的十字。用修正笔（蓝色）改，再对比。";
    CENTRE_TRAINS = "Week 3 · points in space found by construction, not by eye: a face's centre.",
        "第 3 周 · 点定位，用作图而不是凭感觉找点：面的中心。";
    CENTRE_STEP_1 = "Cross the face's diagonals with Sketch", "用草稿笔连出面的两条对角线";
    CENTRE_STEP_2 = "Where they cross is the centre: mark it with Draw", "交点就是中心，用勾线笔标出";
    CENTRE_SAYS = "In perspective the centre lies nearer the far edge than the drawing's middle. \
        Compare: the exact centre in red, ringed when your mark lies more than a pen's width off \
        it. Construction shows the diagonals. Correct in blue and compare again.",
        "透视中，面的真正中心比量出来的正中更靠近远处的边。叠图对比：红色是准确的中心；你的标记偏了一个笔宽以上，中心就画上红圈。点“看画法”，可以看到两条对角线。用修正笔（蓝色）改，再对比。";
    HALVES_TRAINS = "Week 3 · points in space found by construction: a face halved, upper and \
        lower.", "第 3 周 · 点定位，用作图找点：把面上下平分。";
    HALVES_STEP_1 = "The diagonals find the centre", "用对角线找到中心";
    HALVES_STEP_2 = "Through it towards the vanishing point", "过中心连向消失点";
    HALVES_STEP_3 = "The halving line with Draw, edge to edge", "用勾线笔画出平分线，从一边画到另一边";
    HALVES_SAYS = "The last line counts. Compare: the exact line in red. Yours, if it misses the \
        vanishing point, runs on dashed red past the point's ring; if it heads there but misses \
        the centre, the centre is ringed. Construction shows the diagonals and the line to the \
        point. Correct in blue and compare again.",
        "以最后一条线为准。叠图对比：红色是准确的线。你的线没对准消失点，会用红色虚线延长，越过消失点的红圈；对准了消失点却没经过中心，中心就画上红圈。点“看画法”，可以看到对角线和连向消失点的线。用修正笔（蓝色）改，再对比。";
    THIRDS_TRAINS = "Week 3 · points in space found by construction: a face in three equal \
        depths.", "第 3 周 · 点定位，用作图找点：纵深三等分。";
    THIRDS_STEP_1 = "The near upright edge in three", "把近处的竖边三等分";
    THIRDS_STEP_2 = "From the marks to the vanishing point", "从分点连向消失点";
    THIRDS_STEP_3 = "A diagonal crosses them where the uprights stand", "对角线与它们的交点就是竖线的位置";
    THIRDS_STEP_4 = "The two uprights with Draw", "用勾线笔画出两条竖线";
    THIRDS_SAYS = "In perspective the near third is longer than the far one; the last two uprights \
        count. Compare: the exact uprights in red. Where yours stands more than a pen's width off, \
        the point the diagonal finds for it is ringed; one that leans gets its true vertical. \
        Construction shows the near edge's thirds run to the vanishing point and the diagonal. \
        Correct in blue and compare again.",
        "透视中，近的一份比远的一份长；以最后两条竖线为准。叠图对比：红色是准确的竖线。你的竖线偏了一个笔宽以上，对角线上定它的那一点就画上红圈；画歪的竖线会画出真正的竖直线。点“看画法”，可以看到近边三等分点连向消失点的线和对角线。用修正笔（蓝色）改，再对比。";
    STEP_TRAINS = "Week 3 · points in space found by construction: the next face, of the same \
        depth.", "第 3 周 · 点定位，用作图找点：等距扩展出下一个面。";
    STEP_STEP_1 = "The near edge's middle to the vanishing point", "把近边中点连向消失点";
    STEP_STEP_2 = "The top and bottom edges run on", "延长上下两条边";
    STEP_STEP_3 = "From the near top corner through the far edge's middle", "从近处顶角过远边中点画线";
    STEP_STEP_4 = "Where it meets the bottom: the upright, with Draw", "在它与底边的交点处，用勾线笔画出竖边";
    STEP_SAYS = "The last upright counts. Compare: the next face in red. Where your upright stands \
        more than a pen's width off, its foot is ringed: where the line through the far edge's \
        middle meets the bottom edge. Construction shows the lines. Correct in blue and compare \
        again.",
        "以最后一条竖边为准。叠图对比：红色是下一个面。你的竖边偏了一个笔宽以上，它的底端就画上红圈，也就是过远边中点的斜线与底边的交点。点“看画法”，可以看到作图的线。用修正笔（蓝色）改，再对比。";
    MIRROR_TRAINS = "Week 3 · points in space found by construction: a point mirrored across the \
        face's middle.", "第 3 周 · 点定位，用作图找点：关于面中线的对称点。";
    MIRROR_STEP_1 = "The diagonals, and the upright middle through them", "画对角线，再过交点画竖直中线";
    MIRROR_STEP_2 = "The dot's level, and its upright down", "过已知点画水平透视线，再向下画竖线";
    MIRROR_STEP_3 = "The rectangle's diagonals: half its height", "用矩形的对角线找出一半高度";
    MIRROR_STEP_4 = "From the dot through it to the bottom edge", "从已知点经过这个交点，连到底边";
    MIRROR_STEP_5 = "Up to the dot's level: mark it with Draw", "再向上到已知点的高度，用勾线笔标出";
    MIRROR_SAYS = "The mirror lies at the dot's level, on its line to the vanishing point. \
        Compare: the exact point in red, ringed when your mark lies more than a pen's width off \
        it. Construction shows the diagonals, the upright middle and the rectangle. Correct in \
        blue and compare again.",
        "对称点和已知点高度相同，在已知点连向消失点的线上。叠图对比：红色是准确的点；你的标记偏了一个笔宽以上，这个点就画上红圈。点“看画法”，可以看到对角线、竖直中线和矩形。用修正笔（蓝色）改，再对比。";
}
