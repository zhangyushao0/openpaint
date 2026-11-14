//! A colour combination's words: its name and its How card,
//! after V大's method in our own words (squint for the overall tone and cover the frame first, the
//! big areas by colour, each against its neighbours, weak contrasts kept weak), its pens named in
//! the order its hand uses them (S100). In his terms: 色彩组合, 眯眼, 色调, 弱对比, 叠图对比.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("combination", AREAS_EXERCISE, AREAS)];

/// A combination's How names its pens in the order its hand uses them (S100).
const AREAS: How = How {
    heading: AREAS_EXERCISE,
    trains: Some(AREAS_TRAINS),
    steps: &[
        AREAS_STEP_1,
        AREAS_STEP_2,
        AREAS_STEP_3,
        AREAS_STEP_4,
        AREAS_STEP_5,
        COMPARE_STEP,
    ],
    says: &[AREAS_SAYS],
};

texts! { PRACTICE_COMBINATIONS;
    AREAS_EXERCISE = "Colour combination", "色彩组合";
    AREAS_TRAINS = "A corner of a rusted old thing or an abstract painting, a few colours \
        side by side: the overall tone first, each judged against its neighbours.",
        "生锈旧物或抽象画的一角，几块颜色挨在一起：先定整体色调，每块都和相邻的比。";
    AREAS_STEP_1 = "Squint: Fill frame with the overall tone", "眯眼看整体色调，用它填满方框";
    AREAS_STEP_2 = "Big: the largest area first", "用大笔先画最大的色块";
    AREAS_STEP_3 = "Big: each neighbour against it", "用大笔画相邻的色块，逐块和它比";
    AREAS_STEP_4 = "Small: the edges, the weak contrast kept weak", "用小笔收边，弱对比别画强";
    AREAS_STEP_5 = "Soft: only where the picture's edge is soft",
        "柔边笔只用在原图边缘柔和的地方";
    AREAS_SAYS = "Big and Small lay the colour you picked however lightly you press. Compare lays \
        the original over your copy, \
        the slider setting how strongly; each chip, the original's colour over yours, shows its \
        pair on the wheel and in the table, its area outlined in ink; a weak contrast made strong \
        is red. Then the eyedropper compares any point.",
        "大笔、小笔不管压感轻重，画出的都是选中的颜色。叠图对比把原图叠在你的临摹上，浓淡由滑块调节；每个色卡上半是原图的颜色、下半是你的，点一个色卡，这一对颜色就显示在色环和表格里，它的色块用墨线描边；画强了的弱对比标成红色。之后可以用吸管点任意一处来比较。";
}
