//! Value masses on a practice sheet (S96, V大's 黑白灰 and colour blocks grouped by their
//! attributes): the kind's name, its three grey pens, the summary's line, its How card and a whole
//! copy in colour blocks' layer. In his terms: 黑白灰, 色块, 眯眼, 概括.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words(
    "value-masses",
    (VALUE_MASSES_EXERCISE, MASSES_COUNTED),
    VALUE_MASSES,
)];

const VALUE_MASSES: How = How {
    heading: PAINT_VALUE_MASSES,
    trains: Some(MASSES_TRAINS),
    steps: &[MASSES_STEP_1, MASSES_STEP_2, MASSES_STEP_3, COMPARE_STEP],
    says: &[MASSES_SAYS],
};

texts! { PRACTICE_VALUE_MASSES;
    VALUE_MASSES_EXERCISE = "Value masses", "黑白灰";
    PAINT_VALUE_MASSES = "Paint the value masses", "归纳黑白灰";
    DARK_PEN = "Dark", "黑";
    MIDDLE_PEN = "Middle", "灰";
    LIGHT_PEN = "Light", "白";
    MASSES_COUNTED = "Value masses", "黑白灰";
    AT_ITS_VALUE = "At its value", "画对的面积";
    DRILL_NEEDS_MASSES = "Value masses use the composition pack: import it under Practice \
        packs.", "黑白灰要用构图素材包：请在“素材包”下导入。";
    MASSES_TRAINS = "Weeks 2 to 4 · a picture squinted at until only its few big masses are \
        left, each dark, middle or light, grouped by value and not by object.",
        "第 2 至 4 周 · 眯眼看画面，直到只剩几块大色块，每块归为黑、灰、白之一：按明度归纳，不按物体分。";
    MASSES_STEP_1 = "Squint: Fill frame with Middle, the overall tone",
        "眯眼，先用灰填满方框，作为整体色调";
    MASSES_STEP_2 = "The darkest big masses with Dark", "用黑画最暗的几块大色块";
    MASSES_STEP_3 = "Then the light ones with Light", "再用白画亮的色块";
    MASSES_SAYS = "A shadow and a dark ground are one mass; leave out what squinting loses, and \
        paint big, the canvas not zoomed in. Compare lays the answer's three greys over your copy \
        as strongly as the slider sets: its mass edges red, and where your grey differs over more \
        than a pen's width, hatched in the grey it should be. Repaint with the same pens and \
        compare again.",
        "暗部和暗的背景并成一块；眯眼时看不出的就不画；大块地画，不要放大画布。叠图对比把答案的黑白灰叠在你的画上，浓淡由滑块调节：色块的边界画成红线；你的灰画错、且错处宽过一个笔宽的地方，用应有的灰打上斜线。用同样的笔重画，再对比。";
}
