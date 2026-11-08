//! A practice sheet: the bar, the exercise's own tools, the card (each kind's words and How in
//! `practice/`) and the summary, in V大's terms: 叠图对比, 原位, 对齐, 明度, 纯度, 色相; 热身.

use openpaint_app::Pen;
use openpaint_practice::{Axis, Family, Metric};

use super::{Language, Text};

texts! { PRACTICE_TABLE;
    HOW = "How", "怎么练";
    COMPARE = "Compare", "叠图对比";
    NEXT = "Next", "下一个";
    FINISH = "Finish", "结束";
    ONE_MORE = "One more", "再练一次";
    ALL_TOOLS = "All tools", "全部工具";
    SKETCH = "Sketch", "草稿笔";
    DRAW = "Draw", "勾线笔";
    CORRECT = "Correct", "修正笔";
    COLOUR_PICKED = "Colour picked", "当前颜色";
    FILL_FRAME = "Fill frame", "填满方框";
    GOT_IT = "Got it", "知道了";
    IN_PLACE = "In place", "原位";
    ALIGNED = "Aligned", "对齐";
    WARM_UP = "Warm-up", "热身";
    NO_OUTLINE = "This picture has no outline to align.", "这张图没有外轮廓，无法对齐。";
    ALIGNING = "Aligning…", "正在对齐…";
    THIS_SESSION = "this", "本次";
    BEFORE = "before", "以往";
    FIRST_PICK_DELTA = "First pick ΔE", "首次选色 ΔE";
    FINAL_DELTA = "Final ΔE", "最终 ΔE";
    CORRECTIONS = "Corrections", "修正次数";
    DARK = "Dark", "暗色";
    GREYISH = "Greyish", "灰色";
    FIRST_TO_FINAL = "First → final", "首次 → 最终";
}

/// An exercise's pens.
pub(crate) const fn pen(pen: Pen) -> Text {
    match pen {
        Pen::Sketch => SKETCH,
        Pen::Draw => DRAW,
        Pen::Correct => CORRECT,
        Pen::Big => super::BIG,
        Pen::Small => super::SMALL,
        Pen::Soft => super::SOFT,
        Pen::Main => super::MAIN_PEN,
        Pen::Second => super::SECOND_PEN,
        Pen::Third => super::THIRD_PEN,
        Pen::Dark => super::DARK_PEN,
        Pen::Middle => super::MIDDLE_PEN,
        Pen::Light => super::LIGHT_PEN,
    }
}

/// An axis's name on the summary's radar.
pub(crate) const fn judgement_axis(axis: Axis) -> Text {
    match axis {
        Axis::Hue => super::HUE,
        Axis::Chroma => super::CHROMA,
        Axis::Lightness => super::LIGHTNESS,
        Axis::Dark => DARK,
        Axis::LowSaturation => GREYISH,
        Axis::FirstToFinal => FIRST_TO_FINAL,
    }
}

/// A metric's name on the card.
pub(crate) const fn metric(metric: Metric) -> Text {
    match metric {
        Metric::FirstDelta => FIRST_PICK_DELTA,
        Metric::FinalDelta => FINAL_DELTA,
        Metric::FirstHue | Metric::FinalHue => super::HUE,
        Metric::FirstChroma | Metric::FinalChroma => super::CHROMA,
        Metric::FirstLightness | Metric::FinalLightness => super::LIGHTNESS,
        Metric::Corrections => CORRECTIONS,
        Metric::AreasDelta => super::AREAS_DELTA,
        Metric::AreasWeak => super::WEAK_CONTRASTS,
        Metric::AreasMadeStrong => super::WEAK_MADE_STRONG,
        Metric::ScaleOff => super::OFF_EVEN,
        Metric::ScaleUneven => super::UNEVEN_CELLS,
        Metric::ScaleReversed => super::OUT_OF_ORDER,
        Metric::PointAcross => super::OFF_ACROSS,
        Metric::PointDown => super::OFF_DOWN,
        Metric::ProportionOff => super::OFF_ALONG,
        Metric::AngleOff => super::TURNED_OFF,
        Metric::SpaceOff => super::OFF_ON_THE_FACE,
        Metric::SolidOff => super::OFF_THE_SOLID,
        Metric::ObjectOff => super::OFF_THE_OBJECT,
        Metric::DivisionLines(_) => super::LINES_DRAWN,
        Metric::DivisionLength(_) => super::MEAN_LENGTH,
        Metric::MassesMatched => super::AT_ITS_VALUE,
        Metric::KeyLightness => super::LIGHTNESS_LEFT,
        Metric::KeyUnmoved => super::NOT_TOWARD,
        Metric::KeyMerged => super::MERGED_PAIRS,
    }
}

/// A kind's name as its tile and its drill say it: "Line", "Centre" (S91).
pub(crate) fn kind_name(family: Family) -> Text {
    called(family, false)
}

/// A finished drill's heading: "Centre drill done".
pub(crate) fn drill_done(language: Language, family: Family) -> String {
    let name = kind_name(family).get(language);
    match language {
        Language::English => format!("{name} drill done"),
        Language::Chinese => format!("{name}专项练习完成"),
    }
}

/// A drill's name in the practice book: "Centre drill 2".
pub(crate) fn drill_number(language: Language, family: Family, number: u32) -> String {
    let name = kind_name(family).get(language);
    match language {
        Language::English => format!("{name} drill {number}"),
        Language::Chinese => format!("{name}专项 {number}"),
    }
}

/// What an exercise is called ("Line", "Warm-up"): a warm-up's by its part of the session, any
/// other by its kind's words.
fn called(family: Family, warming_up: bool) -> Text {
    if warming_up {
        WARM_UP
    } else {
        super::kind_words(family).name
    }
}

/// The exercise shown, as the bar names it: "Line 3 of 6", "Warm-up 1 of 4".
pub(crate) fn exercise_of(
    language: Language,
    (family, warming_up): (Family, bool),
    (place, of): (usize, usize),
) -> String {
    let called = called(family, warming_up).get(language);
    match language {
        Language::English => format!("{called} {place} of {of}"),
        Language::Chinese => format!("{called} {place}/{of}"),
    }
}

/// A compared exercise's heading: "Line 3", "Warm-up 2".
pub(crate) fn exercise(
    language: Language,
    (family, warming_up): (Family, bool),
    place: usize,
) -> String {
    format!("{} {place}", called(family, warming_up).get(language))
}

/// A finished session's heading.
pub(crate) fn session_done(language: Language, number: u32) -> String {
    match language {
        Language::English => format!("Session {number} done"),
        Language::Chinese => format!("第 {number} 次练习完成"),
    }
}

/// The summary's line per family: "Lines: 8 of 8 compared".
pub(crate) fn compared_of(
    language: Language,
    family: Family,
    (checked, held): (usize, usize),
) -> String {
    super::counted(language, super::kind_words(family).counted, (checked, held))
}
