//! Gamut masks on a practice sheet (S153, Gurney's gamut mask, unit 6): the kind's names, the How
//! card and the reason an area left the mask. In the course's terms: 色域遮罩, 色环, 纯度, 明度,
//! 眯眼, 分得开.

use super::{COMPARE_STEP, How, KindWords, words};
use crate::text::Language;

pub(super) const WORDS: [KindWords; 1] = [words("gamut-mask", GAMUT_EXERCISE, GAMUT_MASK)];

const GAMUT_MASK: How = How {
    heading: PAINT_IN_A_MASK,
    trains: Some(GAMUT_TRAINS),
    steps: &[
        GAMUT_STEP_1,
        GAMUT_STEP_2,
        GAMUT_STEP_3,
        GAMUT_STEP_4,
        COMPARE_STEP,
    ],
    says: &[GAMUT_SAYS],
};

texts! { PRACTICE_GAMUT_MASKS;
    GAMUT_EXERCISE = "Gamut mask", "色域遮罩";
    PAINT_IN_A_MASK = "Paint inside a mask", "色域遮罩";
    GAMUT_TRAINS = "A crop painted again with only the colours inside a mask on the colour \
        wheel, its lightness kept.",
        "把一块局部重画，只用色环上遮罩里的颜色，明度不变。";
    GAMUT_STEP_1 = "Look at the mask: which hues it holds, how pure they may be",
        "先看遮罩：框住了哪些色相，最纯能到多纯";
    GAMUT_STEP_2 = "Squint, then fill the frame with the mask's middle",
        "眯眼看整体，先用遮罩中间的颜色填满方框";
    GAMUT_STEP_3 = "The big areas by their lightness, each colour inside the mask",
        "大块按明度画，每个颜色都在遮罩里";
    GAMUT_STEP_4 = "The small areas last, neighbours still apart",
        "小块最后画，相邻的色块仍要分得开";
    GAMUT_SAYS = "Mix from the colours of the mask's corners. A colour the mask leaves out is \
        moved toward grey or toward a hue the mask holds; its lightness stays.",
        "从遮罩几个角的颜色调起。遮罩外的颜色，往灰里调，或往遮罩里的色相靠；明度不变。";
}

/// An area painted outside the mask (S153), which way it left it (`way`: too pure, too grey, or
/// toward a hue) and its number: "Area 3 is too pure for the mask.", "色块 3 太纯，出了遮罩。".
pub(crate) fn outside_mask(language: Language, way: &str, area: usize) -> String {
    match (language, way) {
        (Language::English, "pure" | "grey") => format!("Area {area} is too {way} for the mask."),
        (Language::English, hue) => format!("Area {area} leans toward {hue}, out of the mask."),
        (Language::Chinese, "pure") => format!("色块 {area} 太纯，出了遮罩。"),
        (Language::Chinese, "grey") => format!("色块 {area} 太灰，出了遮罩。"),
        (Language::Chinese, hue) => {
            format!(
                "色块 {area} {}，出了遮罩。",
                super::super::reasons::leaning(hue)
            )
        }
    }
}

/// Every reason an area outside the mask gives in `language`, for the tables' tests.
#[cfg(test)]
pub(crate) fn made(language: Language) -> Vec<String> {
    [
        "pure", "grey", "red", "yellow", "green", "cyan", "blue", "purple",
    ]
    .map(|way| outside_mask(language, way, 3))
    .to_vec()
}
