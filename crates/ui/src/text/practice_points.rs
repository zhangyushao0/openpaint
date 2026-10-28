//! Stage 3 on a practice sheet (S90): the bar's names, the cards' words and the instructions for
//! the estimates on the flat (V大's 二维抽象网格: points by the frame and long lines, never a grid
//! drawn; proportions by the half and the thirds, compared again and again; angles by an imagined
//! cross or the line run on to the frame's edge) and for points in space (V大's 点定位 and its five
//! tricks: diagonals for the centre, the line to the vanishing point halving, equal parts by the
//! near edge shown true, equal steps by the far edge's middle, a mirror by the rectangle to the
//! middle; every point needs a reason), in our own words. In his terms: 点定位, 二等分, 三等分,
//! 等距延伸, 对称点, 消失点, 对角线, 叠图对比.

use openpaint_practice::{Estimate, Trick};

use super::{Language, Text};

texts! { PRACTICE_POINTS;
    POINTS_STAGE = "Points", "点定位";
    POINT_EXERCISE = "Point", "定点";
    PROPORTION_EXERCISE = "Proportion", "比例";
    ANGLE_EXERCISE = "Angle", "角度";
    CENTRE_EXERCISE = "Centre", "中心";
    HALVES_EXERCISE = "Halves", "二等分";
    THIRDS_EXERCISE = "Thirds", "三等分";
    STEP_EXERCISE = "Next face", "等距延伸";
    MIRROR_EXERCISE = "Mirror", "对称点";
    PLACE_A_POINT = "Place a point", "定点";
    DIVIDE_ALIKE = "Divide alike", "等比分割";
    COPY_AN_ANGLE = "Copy an angle", "复制角度";
    FIND_THE_CENTRE = "Find the face's centre", "找面的中心";
    HALVE_THE_FACE = "Halve the face", "二等分";
    THIRDS_IN_DEPTH = "Thirds in depth", "纵深三等分";
    THE_NEXT_FACE = "The next face", "等距延伸";
    MIRROR_A_POINT = "Mirror a point", "对称点";
    NO_MARK = "No mark found in your lines.", "你的线条里没有找到标记。";
    POINTS_COUNTED = "Points", "定点";
    PROPORTIONS_COUNTED = "Proportions", "比例";
    ANGLES_COUNTED = "Angles", "角度";
    CENTRES_COUNTED = "Centres", "中心";
    HALVES_COUNTED = "Halves", "二等分";
    THIRDS_COUNTED = "Thirds", "三等分";
    STEPS_COUNTED = "Next faces", "等距延伸";
    MIRRORS_COUNTED = "Mirrors", "对称点";
    OFF_ACROSS = "Off across", "横向偏差";
    OFF_DOWN = "Off down", "纵向偏差";
    OFF_ALONG = "Off along", "沿线偏差";
    TURNED_OFF = "Turned off", "角度偏差";
    OFF_ON_THE_FACE = "Off on the face", "面上偏差";
}

texts! { PRACTICE_POINTS_STEPS;
    POINT_1 = "Mark in the empty frame where the dot lies in the frame beside it.",
        "在空白框里标出旁边框中那个点的位置。";
    POINT_2 = "Judge by the frame, never a grid: how far across, how far down? Against the half \
        first, then the thirds; look again and again before you mark.",
        "凭方框判断，不画网格：横向在哪里，纵向在哪里？先和一半比，再和三分之一比；反复对比后再下笔。";
    POINT_3 = "Mark it with Draw, a dot or a small cross: the last mark counts. Sketch is yours \
        for guides.", "用画图笔点一个点或画一个小十字：以最后一个标记为准。草稿笔可以画你自己的辅助线。";
    POINT_4 = "Then tap Compare: the exact point in red, the one read from your mark ringed in \
        blue; dashed pencil runs it across and down to both frames' edges, the halves and thirds \
        ticked on them. Correct in blue and compare again.",
        "然后点“叠图对比”：红色是准确的点，蓝圈是从你的标记读出的点；铅笔虚线把它横向、纵向引到两个方框的边上，边上标出二分之一和三分之一。用修正笔（蓝色）改，再对比。";
    PROPORTION_1 = "Mark the point that divides the segment in the empty frame as the dot divides \
        the segment beside it.", "在空白框的线段上标出一点，使它分割线段的比例和旁边线段上的点相同。";
    PROPORTION_2 = "Compare parts, not lengths: is the dot at the half, a third, a quarter? Nearer \
        which end, by how much of the shorter part? The ringed ends answer each other.",
        "比较的是各段的比例，不是长度：点在一半、三分之一还是四分之一处？靠近哪一端，短的一段占多少？两条线段带圈的端点相对应。";
    PROPORTION_3 = "Mark it with Draw on the segment, a dot or a short tick across it: the last \
        mark counts.", "用画图笔在线段上点一个点，或画一道穿过它的短线：以最后一个标记为准。";
    PROPORTION_4 = "Then tap Compare: the exact place in red across the segment, your mark ringed \
        in blue; both segments' halves and thirds ticked in pencil. Correct in blue and compare \
        again.", "然后点“叠图对比”：红色短线是准确的位置，蓝圈是你的标记；铅笔在两条线段上标出二分之一和三分之一。用修正笔（蓝色）改，再对比。";
    ANGLE_1 = "Draw a line through the dot in the empty frame at the angle of the line beside it.",
        "在空白框里过那个点画一条线，角度和旁边的线条相同。";
    ANGLE_2 = "See the angle against a cross imagined through the line, level and upright; or run \
        the line on to the frame's edges in your mind and see where it meets them.",
        "想象一个过线条的十字（水平和竖直）来比较角度；或在心里把线条延长到方框的边上，看它碰到边的位置。";
    ANGLE_3 = "Draw it with Draw through the dot in one sure stroke: the last line counts.",
        "用画图笔过那个点一笔画成：以最后一条线为准。";
    ANGLE_4 = "Then tap Compare: the exact line in red through the dot, yours dashed in blue; \
        pencil runs each on to the frame's edges, an upright cross through each. Correct in blue \
        and compare again.",
        "然后点“叠图对比”：红色是过点的准确线条，蓝色虚线是你的线；铅笔把它们延长到方框边上，并画出过线条的十字。用修正笔（蓝色）改，再对比。";
    CENTRE_1 = "Mark the centre of the shaded face of the box in the empty frame.",
        "在空白框的盒子上标出阴影面的中心。";
    CENTRE_2 = "In perspective the centre is not the middle of the drawing: it lies nearer the far \
        edge. Cross the face's diagonals with Sketch: they meet at its centre.",
        "透视中，面的中心不在画面的正中：它更靠近远处的边。用草稿笔连出这个面的两条对角线：交点就是中心。";
    CENTRE_3 = "Mark it with Draw, a dot or a small cross: the last mark counts. Every point needs \
        a reason.", "用画图笔点一个点或画一个小十字：以最后一个标记为准。每个点都要有依据。";
    CENTRE_4 = "Then tap Compare: the exact centre in red, your mark ringed in blue; the box beside \
        shows the construction in dashed pencil. Correct in blue and compare again.",
        "然后点“叠图对比”：红色是准确的中心，蓝圈是你的标记；旁边的盒子用铅笔虚线画出作图过程。用修正笔（蓝色）改，再对比。";
    HALVES_1 = "Halve the shaded face, upper and lower: one line towards its vanishing point.",
        "把阴影面分成上下两半：画一条通向消失点的线。";
    HALVES_2 = "Find the centre by the diagonals. The line through it towards the vanishing point, \
        where the face's top and bottom edges meet, halves the face.",
        "先用对角线找到中心。过中心、通向消失点（面的上下两条边延长后的交点）的线，就把这个面平分了。";
    HALVES_3 = "Construct with Sketch, then draw the halving line with Draw from edge to edge: the \
        last line counts.", "用草稿笔作图，再用画图笔从一边画到另一边：以最后一条线为准。";
    HALVES_4 = "Then tap Compare: the exact line in red, yours dashed in blue; the box beside shows \
        the construction in dashed pencil. Correct in blue and compare again.",
        "然后点“叠图对比”：红色是准确的线，蓝色虚线是你的线；旁边的盒子用铅笔虚线画出作图过程。用修正笔（蓝色）改，再对比。";
    THIRDS_1 = "Divide the shaded face into three equal depths: two upright lines.",
        "把阴影面在纵深上三等分：画两条竖线。";
    THIRDS_2 = "In perspective the near third is longer than the far one. The near upright edge \
        shows true: divide it in three and run lines from the marks to the vanishing point.",
        "透视中近的一份比远的一份长。近处的竖边没有透视变形：把它三等分，从分点向消失点连线。";
    THIRDS_3 = "A diagonal of the face crosses those lines where the uprights stand. Construct with \
        Sketch, draw the two uprights with Draw: the last two count.",
        "面的一条对角线与这些线的交点，就是竖线的位置。用草稿笔作图，用画图笔画出两条竖线：以最后两条为准。";
    THIRDS_4 = "Then tap Compare: the exact uprights in red, yours dashed in blue; the box beside \
        shows the construction in dashed pencil. Correct in blue and compare again.",
        "然后点“叠图对比”：红色是准确的竖线，蓝色虚线是你的线；旁边的盒子用铅笔虚线画出作图过程。用修正笔（蓝色）改，再对比。";
    STEP_1 = "Draw the next face beyond the shaded one, of the same depth: its far upright edge.",
        "在阴影面后面接着画一个同样纵深的面：画出它远处的竖边。";
    STEP_2 = "Find the middle of the face's far edge: the line from the near edge's middle towards \
        the vanishing point meets it there.",
        "先找远处竖边的中点：从近处竖边的中点向消失点连线，与远边的交点就是。";
    STEP_3 = "A line from the near top corner through that middle meets the bottom edge, run on, \
        at the next face's far corner: stand the upright there with Draw. Sketch for the \
        construction.", "从近处的顶角过这个中点画线，与延长的底边相交处就是下一个面的远角：用画图笔在那里画竖边。作图用草稿笔。";
    STEP_4 = "Then tap Compare: the next face in red, your upright dashed in blue; the box beside \
        shows the construction in dashed pencil. Correct in blue and compare again.",
        "然后点“叠图对比”：红色是下一个面，蓝色虚线是你的竖边；旁边的盒子用铅笔虚线画出作图过程。用修正笔（蓝色）改，再对比。";
    MIRROR_1 = "Mark the point that mirrors the dot across the shaded face's upright middle.",
        "标出阴影面上那个点关于面的竖直中线的对称点。";
    MIRROR_2 = "The upright middle stands at the centre, where the diagonals cross. The mirror lies \
        at the dot's height: on the line from the dot to the vanishing point.",
        "竖直中线过中心（对角线的交点）。对称点和原点高度相同：在原点通向消失点的线上。";
    MIRROR_3 = "Build the rectangle between the dot and the middle down to the bottom edge: its \
        diagonals find the middle's half height. The line from the dot through it meets the \
        bottom edge under the mirror. Mark it with Draw.",
        "在原点和中线之间向下作出到底边的矩形：它的对角线找出中线一半的高度。从原点过这一点画线，与底边的交点正对着对称点。用画图笔标出。";
    MIRROR_4 = "Then tap Compare: the exact point in red, your mark ringed in blue; the box beside \
        shows the construction in dashed pencil. Correct in blue and compare again.",
        "然后点“叠图对比”：红色是准确的点，蓝圈是你的标记；旁边的盒子用铅笔虚线画出作图过程。用修正笔（蓝色）改，再对比。";
}

/// An estimate's instructions: the heading and its paragraphs, the last saying once what Compare
/// shows.
pub(crate) const fn estimate_steps(estimate: Estimate) -> (Text, &'static [Text]) {
    match estimate {
        Estimate::Point => (PLACE_A_POINT, &[POINT_1, POINT_2, POINT_3, POINT_4]),
        Estimate::Proportion => (
            DIVIDE_ALIKE,
            &[PROPORTION_1, PROPORTION_2, PROPORTION_3, PROPORTION_4],
        ),
        Estimate::Angle => (COPY_AN_ANGLE, &[ANGLE_1, ANGLE_2, ANGLE_3, ANGLE_4]),
    }
}

/// A trick's instructions, likewise.
pub(crate) const fn trick_steps(trick: Trick) -> (Text, &'static [Text]) {
    match trick {
        Trick::Centre => (FIND_THE_CENTRE, &[CENTRE_1, CENTRE_2, CENTRE_3, CENTRE_4]),
        Trick::Halves => (HALVE_THE_FACE, &[HALVES_1, HALVES_2, HALVES_3, HALVES_4]),
        Trick::Thirds => (THIRDS_IN_DEPTH, &[THIRDS_1, THIRDS_2, THIRDS_3, THIRDS_4]),
        Trick::Step => (THE_NEXT_FACE, &[STEP_1, STEP_2, STEP_3, STEP_4]),
        Trick::Mirror => (MIRROR_A_POINT, &[MIRROR_1, MIRROR_2, MIRROR_3, MIRROR_4]),
    }
}

/// The summary's line for a family counted by its name: "Points: 2 of 2 compared".
pub(crate) fn counted(language: Language, name: Text, (checked, held): (usize, usize)) -> String {
    match language {
        Language::English => format!("{}: {checked} of {held} compared", name.get(language)),
        Language::Chinese => format!("{}：已对比 {checked}/{held}", name.get(language)),
    }
}
