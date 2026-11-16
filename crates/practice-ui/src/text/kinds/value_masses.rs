//! Value masses on a practice sheet (S96, V大's 黑白灰 and colour blocks grouped by their
//! attributes): the kind's name, its three grey pens, its How card and a whole
//! copy in colour blocks' layer. In his terms: 黑白灰, 色块, 眯眼, 概括. And its two-value variant,
//! Notan (S156, 黑白构图): each mass decided light or dark, no middle.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 2] = [
    words("value-masses", VALUE_MASSES_EXERCISE, VALUE_MASSES),
    words("notan", NOTAN_EXERCISE, NOTAN),
];

const VALUE_MASSES: How = How {
    heading: PAINT_VALUE_MASSES,
    trains: Some(MASSES_TRAINS),
    steps: &[MASSES_STEP_1, MASSES_STEP_2, MASSES_STEP_3, COMPARE_STEP],
    says: &[MASSES_SAYS],
};
const NOTAN: How = How {
    heading: PAINT_NOTAN,
    trains: Some(NOTAN_TRAINS),
    steps: &[NOTAN_STEP_FILL, NOTAN_STEP_DARK, COMPARE_STEP],
    says: &[NOTAN_SAYS],
};

texts! { PRACTICE_VALUE_MASSES;
    VALUE_MASSES_EXERCISE = "Value masses", "黑白灰";
    PAINT_VALUE_MASSES = "Paint the value masses", "归纳黑白灰";
    DARK_PEN = "Dark", "黑";
    MIDDLE_PEN = "Middle", "灰";
    LIGHT_PEN = "Light", "白";
    MASSES_TRAINS = "A picture squinted at until only its few big masses are \
        left, each dark, middle or light, grouped by value and not by object.",
        "眯眼看画面，直到只剩几块大色块，每块归为黑、灰、白之一：按明度归纳，不按物体分。";
    MASSES_STEP_1 = "Squint: Fill frame with Middle, the overall tone",
        "眯眼，先用灰填满方框，作为整体色调";
    MASSES_STEP_2 = "The darkest big masses with Dark", "用黑画最暗的几块大色块";
    MASSES_STEP_3 = "Then the light ones with Light", "再用白画亮的色块";
    MASSES_SAYS = "A shadow and a dark ground are one mass; leave out what squinting loses, and \
        paint big, the canvas not zoomed in. Repaint a mass with the same pens.",
        "暗部和暗的背景并成一块；眯眼时看不出的就不画；大块地画，不要放大画布。要改哪一块，就用同样的笔重画。";
    NOTAN_EXERCISE = "Notan", "黑白构图";
    PAINT_NOTAN = "Paint the picture in two values", "用黑白两个明度归纳画面";
    NOTAN_TRAINS = "Every big mass decided as light or dark, no middle: the picture's design in \
        two values.",
        "每一块大色块都要决定是亮还是暗，没有中间调：用两个明度看出画面的大结构。";
    NOTAN_STEP_FILL = "Squint: Fill frame with Light", "眯眼，先用白填满方框";
    NOTAN_STEP_DARK = "The dark masses with Dark, each one decided", "用黑画暗的色块，每一块都做决定";
    NOTAN_SAYS = "Where a mass sits between, choose: put it with the darks or with the lights, \
        whichever keeps the big shapes clear. More than one choice can be right.",
        "介于中间的色块要做选择：并进暗还是并进亮，看哪样大形更清楚。分法不止一种对的。";
}
