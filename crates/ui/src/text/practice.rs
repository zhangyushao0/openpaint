//! A practice sheet: the bar, the exercise's own tools, the card (each kind's How in
//! `practice_demo.rs`) and the summary, in V大's terms: 叠图对比, 原位, 对齐, 明度, 纯度, 色相;
//! 组合线条, 热身.

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
    DRAW = "Draw", "画图笔";
    CORRECT = "Correct", "修正笔";
    COLOUR_PICKED = "Colour picked", "所选颜色";
    FILL_FRAME = "Fill frame", "填满方框";
    GOT_IT = "Got it", "知道了";
    IN_PLACE = "In place", "原位";
    ALIGNED = "Aligned", "对齐";
    WARM_UP = "Warm-up", "热身";
    LINE = "Line", "线条";
    COMBINED_LINES = "Combined lines", "组合线条";
    COPY_A_LINE = "Copy a line", "单根线条";
    COPY_COMBINED = "Copy combined lines", "组合线条";
    MATCH_A_COLOUR = "Match a colour", "单色";
    SHAPE_EXERCISE = "Shape", "单体图形";
    COPY_A_SHAPE = "Copy a single shape", "单体图形";
    NO_OUTLINE = "This picture has no outline to align.", "这张图没有外形轮廓，不能对齐。";
    ALIGNING = "Aligning…", "正在对齐…";
    THIS_SESSION = "this", "本次";
    BEFORE = "before", "此前";
    COLOURS = "Colours", "颜色";
    FIRST_PICK_DELTA = "First pick ΔE", "首次选色 ΔE";
    FINAL_DELTA = "Final ΔE", "最终 ΔE";
    CORRECTIONS = "Corrections", "修正次数";
    DARK = "Dark", "暗色";
    GREYISH = "Greyish", "偏灰";
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
    }
}

/// A kind's name as its tile and its drill say it: "Line", "Centre" (S91).
pub(crate) const fn kind_name(family: Family) -> Text {
    called(family, false)
}

/// A finished drill's heading: "Centre drill done".
pub(crate) fn drill_done(language: Language, family: Family) -> String {
    let name = kind_name(family).get(language);
    match language {
        Language::English => format!("{name} drill done"),
        Language::Chinese => format!("{name}专项完成"),
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
/// other by its family.
const fn called(family: Family, warming_up: bool) -> Text {
    match (warming_up, family) {
        (true, _) => WARM_UP,
        (false, Family::Line) => LINE,
        (false, Family::Combined) => COMBINED_LINES,
        (false, Family::Shape) => SHAPE_EXERCISE,
        (false, Family::Box) => super::BOX_EXERCISE,
        (false, Family::FreeBoxes) => super::FREE_BOXES_EXERCISE,
        (false, Family::Ellipse) => super::ELLIPSE_EXERCISE,
        (false, Family::Point) => super::POINT_EXERCISE,
        (false, Family::Proportion) => super::PROPORTION_EXERCISE,
        (false, Family::Angle) => super::ANGLE_EXERCISE,
        (false, Family::Centre) => super::CENTRE_EXERCISE,
        (false, Family::Halves) => super::HALVES_EXERCISE,
        (false, Family::Thirds) => super::THIRDS_EXERCISE,
        (false, Family::Step) => super::STEP_EXERCISE,
        (false, Family::Mirror) => super::MIRROR_EXERCISE,
        (false, Family::Cylinder) => super::CYLINDER_EXERCISE,
        (false, Family::Cone) => super::CONE_EXERCISE,
        (false, Family::Sphere) => super::SPHERE_EXERCISE,
        (false, Family::Object) => super::OBJECT_EXERCISE,
        (false, Family::Division) => super::DIVISION_EXERCISE,
        (false, Family::WholeCopy) => super::WHOLE_COPY_EXERCISE,
        (false, Family::ValueMasses) => super::VALUE_MASSES_EXERCISE,
        (false, Family::Colour) => super::COLOUR,
        (false, Family::Areas) => super::AREAS_EXERCISE,
        (false, Family::Scale) => super::SCALE_EXERCISE,
        (false, Family::Study) => super::STUDY_EXERCISE,
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
    let named = |name: Text| super::counted(language, name, (checked, held));
    match (language, family) {
        (_, Family::Point) => named(super::POINTS_COUNTED),
        (_, Family::Proportion) => named(super::PROPORTIONS_COUNTED),
        (_, Family::Angle) => named(super::ANGLES_COUNTED),
        (_, Family::Centre) => named(super::CENTRES_COUNTED),
        (_, Family::Halves) => named(super::HALVES_COUNTED),
        (_, Family::Thirds) => named(super::THIRDS_COUNTED),
        (_, Family::Step) => named(super::STEPS_COUNTED),
        (_, Family::Mirror) => named(super::MIRRORS_COUNTED),
        (_, Family::Cylinder) => named(super::CYLINDERS_COUNTED),
        (_, Family::Cone) => named(super::CONES_COUNTED),
        (_, Family::Sphere) => named(super::SPHERES_COUNTED),
        (_, Family::Object) => named(super::OBJECTS_COUNTED),
        (_, Family::Division) => named(super::DIVISIONS_COUNTED),
        (_, Family::WholeCopy) => named(super::WHOLE_COPIES_COUNTED),
        (_, Family::ValueMasses) => named(super::MASSES_COUNTED),
        (Language::English, Family::Line) => format!("Lines: {checked} of {held} compared"),
        (Language::English, Family::Combined) => {
            format!("Combined lines: {checked} of {held} compared")
        }
        (Language::English, Family::Colour) => format!("Colours: {checked} of {held} compared"),
        (Language::English, Family::Shape) => format!("Shapes: {checked} of {held} compared"),
        (Language::English, Family::Box) => format!("Boxes: {checked} of {held} compared"),
        (Language::English, Family::FreeBoxes) => {
            format!("Free boxes: {checked} of {held} compared")
        }
        (Language::English, Family::Ellipse) => format!("Ellipses: {checked} of {held} compared"),
        (Language::English, Family::Areas) => {
            format!("Colour combinations: {checked} of {held} compared")
        }
        (Language::English, Family::Scale) => {
            format!("Value scales: {checked} of {held} compared")
        }
        (Language::English, Family::Study) => {
            format!("Colour copies: {checked} of {held} compared")
        }
        (Language::Chinese, Family::Line) => format!("线条：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Combined) => format!("组合线条：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Shape) => format!("单体图形：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Box) => format!("透视盒子：已对比 {checked}/{held}"),
        (Language::Chinese, Family::FreeBoxes) => format!("自由盒子：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Ellipse) => format!("椭圆：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Colour) => format!("颜色：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Areas) => format!("色彩组合：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Scale) => format!("明度推移：已对比 {checked}/{held}"),
        (Language::Chinese, Family::Study) => format!("色彩临摹：已对比 {checked}/{held}"),
    }
}
