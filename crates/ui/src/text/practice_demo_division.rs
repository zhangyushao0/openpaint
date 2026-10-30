//! A division's and a whole copy's How cards (`practice_demo.rs`, S95), after V大's method in our
//! own words: the most obvious divisions found by feel, squinting, never by analysis; at most five
//! main lines, as long as they run, small turns ignored, a bent line's turning point first, about
//! half the time spent on them; from them the second level, then the third; no right or wrong, but
//! roughly what others see; the canvas not zoomed in. Then the division dimmed and the whole
//! picture copied over it at low completion, the big shapes right, small errors fine. In his
//! terms: 画面分割, 主分割, 次分割, 三级, 整图复制, 眯眼.

use super::practice_demo::{COMPARE_STEP, How};

pub(crate) const DIVISION: How = How {
    heading: super::DIVIDE_A_PICTURE,
    trains: Some(DIVISION_TRAINS),
    steps: &[
        DIVISION_STEP_1,
        DIVISION_STEP_2,
        DIVISION_STEP_3,
        COMPARE_STEP,
    ],
    says: &[DIVISION_SAYS],
};

pub(crate) const WHOLE_COPY: How = How {
    heading: super::COPY_A_WHOLE_PICTURE,
    trains: Some(WHOLE_COPY_TRAINS),
    steps: &[WHOLE_COPY_STEP_1, WHOLE_COPY_STEP_2, COMPARE_STEP],
    says: &[WHOLE_COPY_SAYS],
};

texts! { PRACTICE_DEMO_DIVISION;
    DIVISION_TRAINS = "Weeks 3 and 4 · a whole picture's few big divisions found by squinting, \
        drawn beside it in three levels.",
        "第 3、4 周 · 眯眼找出整幅画最明显的几条分割线，在图旁分三级画出。";
    DIVISION_STEP_1 = "Squint: the few most obvious lines, red, right through",
        "眯眼：最明显的几条主分割线，红色，尽量贯穿画面";
    DIVISION_STEP_2 = "From them, each part's frame, green", "由主分割线找出各部分的外框，绿色";
    DIVISION_STEP_3 = "Shorter still, blue", "更短的三级分割线，蓝色";
    DIVISION_SAYS = "Spend about half the time on the red lines: at most five, as long as they \
        run, small turns ignored, a bent line's turning point found first; the canvas not zoomed \
        in. There is no right or wrong, but others see roughly the same. Red, green and blue are \
        the three levels; Compare lays the picture faint under your lines, and the table counts \
        each level's lines and how far they run on average.",
        "一半左右的时间花在红色主分割线上：最多五条，尽量长，忽略小的转折，有弯折的长线先找转折点；不要放大画布。分割没有对错，但大家找到的大致相同。红、绿、蓝是三级分割；叠图对比把原图淡淡地放在你的线条下面，表格列出每一级的条数和平均长度。";
    WHOLE_COPY_TRAINS = "Weeks 3 and 4 · a division continued into a copy of the whole picture: \
        the big shapes right, no details.",
        "第 3、4 周 · 在画面分割的基础上整图复制：大形准确，不画细节。";
    WHOLE_COPY_STEP_1 = "Divide it: red, then green", "先分割：先红后绿";
    WHOLE_COPY_STEP_2 = "Draw: the big shapes over it, no details",
        "画图笔：在上面画出大形，不画细节";
    WHOLE_COPY_SAYS = "Choose Draw and your division dims under the line drawing, as a lower \
        layer's opacity would. Compare lays the picture over your copy, your division hidden; the \
        slider sets how strongly it shows. The big shapes should be right; small local errors \
        are fine.",
        "选画图笔后，你的分割线会变淡，留在线稿下面，就像降低了图层的不透明度。叠图对比把原图叠在你的临摹上，分割线隐藏；滑块调节原图的浓淡。大形要准，局部的小误差没关系。";
}
