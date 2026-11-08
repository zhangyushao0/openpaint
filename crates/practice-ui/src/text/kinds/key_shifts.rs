//! Key shifts on a practice sheet (S107, V大's 变调, week 5): the kind's and the stage's names, the
//! tendency's label, the card's lines after Compare (one per check that failed), the Grey view,
//! the locked original's notice and the How card. In his terms: 变调, 倾向色, 明度, 纯度, 偏冷,
//! 偏暖, 对比色, 看黑白, 分得开.

use super::{COMPARE_STEP, How, KindWords, words};
use crate::text::Language;

pub(super) const WORDS: [KindWords; 1] = [words(
    "key-shift",
    (KEY_SHIFT_EXERCISE, KEY_SHIFTS_COUNTED),
    KEY_SHIFT,
)];

const KEY_SHIFT: How = How {
    heading: SHIFT_THE_KEY,
    trains: Some(KEY_TRAINS),
    steps: &[
        KEY_STEP_1,
        KEY_STEP_2,
        KEY_STEP_3,
        KEY_STEP_4,
        KEY_STEP_5,
        KEY_STEP_6,
        COMPARE_STEP,
    ],
    says: &[KEY_SAYS],
};

texts! { PRACTICE_KEY_SHIFTS;
    KEY_SHIFT_EXERCISE = "Key shift", "变调";
    KEY_SHIFTS_STAGE = "Key shifts", "变调";
    KEY_SHIFTS_COUNTED = "Key shifts", "变调";
    TENDENCY = "Tendency", "倾向色";
    GREY_VIEW = "Grey", "看黑白";
    ORIGINAL_LOCKED = "A key shift's original can't be picked: judge its colours by eye.",
        "变调的原图不能吸色：用眼睛判断它的颜色。";
    LIGHTNESS_LEFT = "Lightness left", "明度偏离";
    NOT_TOWARD = "Not toward the tendency", "没往倾向色走";
    MERGED_PAIRS = "Merged", "混成一块";
    SHIFT_THE_KEY = "Shift the key", "变调";
    KEY_TRAINS = "Week 5 · a crop painted again as if seen in another light: every colour moved \
        toward a tendency, its lightness kept.",
        "第 5 周 · 把一块局部重画成换了一种光线的样子：每个颜色都往倾向色偏，明度不变。";
    KEY_STEP_1 = "Look at the tendency: warm or cool, pure or grey",
        "先看倾向色：偏暖还是偏冷，纯还是灰";
    KEY_STEP_2 = "Fill frame: the largest colour first, the whole picture shifted",
        "先用最大的颜色填满方框，整体先偏过去";
    KEY_STEP_3 = "Greys move most and gain a little purity", "灰色偏得最多，纯度略微提高";
    KEY_STEP_4 = "The tendency's opposites move a little and turn greyer",
        "倾向色的对比色只偏一点，变灰";
    KEY_STEP_5 = "The lightest barely move; every lightness kept",
        "最亮的几乎不动；每块的明度都不变";
    KEY_STEP_6 = "After the shift, the areas still apart", "偏完各色仍要分得开";
    KEY_SAYS = "Hues near the tendency draw closer to it, the short way round the colour wheel. \
        The original can't be picked; your own paint can. A key shift has no answer: Compare \
        marks only what is off, red on your copy and on its swatch: an area whose lightness left \
        the original's, one that did not move toward the tendency (a grey too: it should move \
        most), two areas apart in the original that merged. Grey shows both frames in grey, only \
        the lightness marked. On the disc each original colour is a ring and yours a dot; the \
        triangle is the range the tendency allows: across from it only greyed colours.",
        "与倾向色相近的色相向它靠拢，沿色环走近的那一边。原图不能吸色，你自己画的可以。变调没有标准答案：叠图对比只标出不对的地方，在你的画和色卡上用红色标出：明度偏离原图的色块，没往倾向色走的色块（灰色也算：它应该偏得最多），原图里分得开、你画得混成一块的两块颜色。“看黑白”把两边都变成黑白，只标明度。圆盘上，原图的每个颜色是一个圆圈，你的是一个圆点；三角形是倾向色允许的范围：它对面只能用灰下来的颜色。";
}

/// A key shift compared: areas lighter than the original's (`lighter`), darker: "1 area darker",
/// "1 块明度偏暗".
pub(crate) fn lightness_off(language: Language, count: usize, lighter: bool) -> String {
    let plural = if count == 1 { "" } else { "s" };
    match (language, lighter) {
        (Language::English, true) => format!("{count} area{plural} lighter"),
        (Language::English, false) => format!("{count} area{plural} darker"),
        (Language::Chinese, true) => format!("{count} 块明度偏亮"),
        (Language::Chinese, false) => format!("{count} 块明度偏暗"),
    }
}

/// Areas that did not move toward the tendency: "1 area not toward the tendency".
pub(crate) fn not_toward(language: Language, count: usize) -> String {
    let plural = if count == 1 { "" } else { "s" };
    match language {
        Language::English => format!("{count} area{plural} not toward the tendency"),
        Language::Chinese => format!("{count} 块没往倾向色走"),
    }
}

/// Areas apart in the original merged into `blocks`: "2 areas merged into one".
pub(crate) fn merged(language: Language, areas: usize, blocks: usize) -> String {
    match (language, blocks) {
        (Language::English, 1) => format!("{areas} areas merged into one"),
        (Language::English, _) => format!("{areas} areas merged into {blocks}"),
        (Language::Chinese, 1) => format!("{areas} 块混成了一块"),
        (Language::Chinese, _) => format!("{areas} 块混成了 {blocks} 块"),
    }
}
