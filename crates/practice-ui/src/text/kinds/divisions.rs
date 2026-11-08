//! A division's words (S95): its name, its summary's line and its How card, after V大's method in our
//! own words: the most obvious divisions found by feel, squinting, never by analysis; at most five
//! main lines, as long as they run, small turns ignored, a bent line's turning point first, about
//! half the time spent on them; from them the second level, then the third; no right or wrong, but
//! roughly what others see; the canvas not zoomed in. In his terms: 画面分割, 主分割, 次分割,
//! 三级分割, 眯眼.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words(
    "division",
    (DIVISION_EXERCISE, DIVISIONS_COUNTED),
    DIVISION,
)];

const DIVISION: How = How {
    heading: DIVIDE_A_PICTURE,
    trains: Some(DIVISION_TRAINS),
    steps: &[
        DIVISION_STEP_1,
        DIVISION_STEP_2,
        DIVISION_STEP_3,
        COMPARE_STEP,
    ],
    says: &[DIVISION_SAYS],
};

texts! { PRACTICE_DIVISIONS;
    DIVISION_EXERCISE = "Division", "画面分割";
    DIVIDE_A_PICTURE = "Divide a picture", "画面分割";
    DIVISIONS_COUNTED = "Divisions", "画面分割";
    DIVISION_TRAINS = "Weeks 3 and 4 · a whole picture's few big divisions found by squinting, \
        drawn beside it in three levels.",
        "第 3、4 周 · 眯眼找出整幅画最明显的几条分割线，在图旁分三级画出。";
    DIVISION_STEP_1 = "Squint: the few most obvious lines, red, right through",
        "眯眼找最明显的主分割线，用红色贯穿画面";
    DIVISION_STEP_2 = "From them, each part's frame, green", "由主分割线找出各部分的外框，用绿色";
    DIVISION_STEP_3 = "Shorter still, blue", "再画更短的三级分割线，用蓝色";
    DIVISION_SAYS = "Spend about half the time on the red lines: at most five, as long as they \
        run, small turns ignored, a bent line's turning point found first; the canvas not zoomed \
        in. There is no right or wrong, but others see roughly the same. Red, green and blue are \
        the three levels; Compare lays the picture under your lines, the slider sets how \
        strongly it shows, and the table counts each level's lines and how far they run on \
        average.",
        "一半左右的时间花在红色主分割线上：最多五条，尽量长，忽略小的转折，有弯折的长线先找转折点；不要放大画布。分割没有对错，但大家找到的大致相同。红、绿、蓝分别是主分割、次分割和三级分割；叠图对比把原图放在你的线条下面，滑块调节原图浓淡，表格列出每一级的条数和平均长度。";
}
