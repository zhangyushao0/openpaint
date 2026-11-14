//! A whole copy's words (S95): its name and its How card, after V大's method in
//! our own words: the division dimmed and the whole picture copied over it at low completion, the
//! big shapes right, small errors fine. In his terms: 整图复制, 画面分割, 眯眼.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("whole-copy", WHOLE_COPY_EXERCISE, WHOLE_COPY)];

const WHOLE_COPY: How = How {
    heading: COPY_A_WHOLE_PICTURE,
    trains: Some(WHOLE_COPY_TRAINS),
    steps: &[WHOLE_COPY_STEP_1, WHOLE_COPY_STEP_2, COMPARE_STEP],
    says: &[WHOLE_COPY_SAYS],
};

texts! { PRACTICE_WHOLE_COPIES;
    WHOLE_COPY_EXERCISE = "Whole copy", "整图复制";
    COPY_A_WHOLE_PICTURE = "Copy a whole picture", "整图复制";
    WHOLE_COPY_TRAINS = "A division continued into a copy of the whole picture: \
        the big shapes right, no details.",
        "在画面分割的基础上整图复制：大形准确，不画细节。";
    WHOLE_COPY_STEP_1 = "Divide it: red, then green", "先分割：先红后绿";
    WHOLE_COPY_STEP_2 = "Draw: the big shapes over it, no details",
        "用勾线笔在上面画出大形，不画细节";
    WHOLE_COPY_SAYS = "Draw the line drawing over your division, dimmed under it; or, if your \
        shapes are strong, paint it in colour blocks over the division: Fill frame for the overall tone, Big for the big blocks, Small for edges \
        and small areas, Soft only where the picture's own edge or gradient is soft. Compare lays \
        the picture over your copy, your division hidden; the slider sets how strongly it shows, \
        and the eyedropper compares a point of your blocks. The big shapes should be right; small \
        local errors are fine.",
        "用勾线笔在变淡的分割线上画线稿；抓形能力强的话，也可以在分割线上画色块：用“填满方框”铺整体色调，大笔画大色块，小笔收边、画小色块，柔边笔只用在原图边缘或渐变柔和的地方。叠图对比把原图叠在你的临摹上，分割线隐藏；滑块调节原图浓淡，吸管可以对比色块上的一点。大形要准，局部的小误差没关系。";
}
