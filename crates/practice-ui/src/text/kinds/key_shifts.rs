//! Key shifts on a practice sheet (S107, V大's 变调, week 5): the kind's names, the
//! tendency's label, the card's lines after Compare (one per check that failed), the Grey view
//! and the How card. In his terms: 变调, 倾向色, 明度, 纯度, 偏冷,
//! 偏暖, 对比色, 看黑白, 分得开.

use super::{COMPARE_STEP, How, KindWords, words};
use crate::text::Language;

pub(super) const WORDS: [KindWords; 1] = [words("key-shift", KEY_SHIFT_EXERCISE, KEY_SHIFT)];

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
    TENDENCY = "Tendency", "倾向色";
    GREY_VIEW = "Grey", "看黑白";
    SHIFT_THE_KEY = "Shift the key", "变调";
    KEY_TRAINS = "A crop painted again as if seen in another light: every colour moved \
        toward a tendency, its lightness kept.",
        "把一块局部重画成换了一种光线的样子：每个颜色都往倾向色偏，明度不变。";
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
    KEY_SAYS = "Hues near the tendency draw closer to it, the short way round the colour wheel.",
        "与倾向色相近的色相向它靠拢，沿色环走近的那一边。";
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
