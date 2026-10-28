//! A practice sheet: the bar, the exercise's own tools, the card (instructions after V大's method,
//! in our own words, in his terms: 叠图对比, 原位, 对齐, 更红/更黄, 更亮/更暗, 更纯/更灰, 明度,
//! 纯度, 色相; 组合线条, 热身; each family's How saying once what Compare shows, D108) and the
//! summary.

use openpaint_app::Pen;
use openpaint_practice::{Axis, Estimate, Family, Metric, Trick};

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

texts! { PRACTICE_STEPS;
    LINE_1 = "Copy the line into the empty frame beside it.", "把线条临摹到旁边的空白框里。";
    LINE_2 = "Look before you draw: find its two ends and where it bends most, and judge where \
        they sit in the frame. Mark them lightly with Sketch if you like.",
        "先看再画：找出两个端点和弯得最厉害的地方，判断它们在框里的位置。可以先用草稿笔轻轻标出来。";
    LINE_3 = "Draw the line with Draw in one sure stroke. If it goes wrong, don't erase: draw the \
        better line over it.",
        "用画图笔一笔果断画出。画错了不要擦，在上面画一条更准的。";
    LINE_4 = "Then tap Compare: the original lies over your line in red, in place or aligned \
        (its shape alone). Correct in blue and compare again.",
        "然后点“叠图对比”：原线条以红色叠在你的线条上，原位或对齐（只比形状）。用修正笔（蓝色）改，再对比，次数不限。";
    GROUP_1 = "Copy the group of lines into the empty frame beside it.",
        "把这组线条临摹到旁边的空白框里。";
    GROUP_2 = "See the whole group as one shape first: its outer ends and corners, where lines \
        meet or cross. Fix those key points in the frame before anything else, lightly with \
        Sketch.",
        "先把整组线条看成一个形状：外围的端点和转角，线条相接或交叉的地方。先用草稿笔轻轻定下这些关键点在框里的位置。";
    GROUP_3 = "Then the inside: each line between its points, its bends found as for a single \
        line, in sure strokes with Draw. Don't erase; draw the better line over it.",
        "再画里面：每条线连接它的关键点，弯曲处像单根线条一样找。用画图笔果断画出，不要擦，画错就在上面画一条更准的。";
    GROUP_4 = "Compare, don't guess: tap Compare. The group lies over yours in red: In place \
        shows where it lies; Aligned lays it onto yours as one shape. Correct in blue and \
        compare again.",
        "要对比，别凭感觉画：点“叠图对比”。整组原线条以红色叠在你的线条上：原位看位置，对齐把整组线条当成一个形状叠到你的线条上。用修正笔（蓝色）改，再对比。";
    COLOUR_1 = "Fill the empty frame with the colour of the swatch beside it.",
        "用旁边色块的颜色填满空白框。";
    COLOUR_2 = "Name it first: its hue, how pure or grey, how light or dark. Then pick it once by \
        eye on the wheel and tap Fill frame.",
        "先说出它的色相、纯度和明暗：偏纯还是偏灰，偏亮还是偏暗。再凭眼睛在色轮上选一次，点“填满方框”。";
    COLOUR_3 = "Compare by eye: redder or yellower? Lighter or darker? Purer or greyer? Pick and \
        fill again two or three times.",
        "用眼睛比较：更红还是更黄？更亮还是更暗？更纯还是更灰？再选色填充两三次。";
    COLOUR_4 = "Then tap Compare: on the wheel yours is ringed in ink, the original in red. Under \
        it ΔE, how different they look (about 2 just noticeable), and its lightness, chroma and \
        hue, each a bar from the original's red line to yours; correct the longest first. The \
        swatch can't be picked before.",
        "然后点“叠图对比”：色轮上墨色圈是你的颜色，红色圈是原色。下面 ΔE 是看起来差多少（约 2 刚能看出），再分明度、纯度、色相，每行一条从原色红线到你的颜色的横条：先改最长的。对比之前，吸管吸不到色块。";
    COLOUR_5 = "The summary's shape is your colour judgement: the rim exact, your earlier \
        sessions dashed.", "练习结束时的图形是你的颜色判断力：外圈为完全准确，虚线是你此前的练习。";
    SHAPE_1 = "Copy the object's outline into the empty frame beside it as a line drawing: not \
        too fine, but accurate.",
        "把物体的外形用线条临摹到旁边的空白框里：不必画得很细，但要准。";
    SHAPE_2 = "See the whole silhouette first. Mark where its top, bottom, left and right ends sit \
        in the frame, lightly with Sketch.",
        "先看整体轮廓。用草稿笔轻轻标出它最上、最下、最左、最右的位置在框里的哪里。";
    SHAPE_3 = "Hierarchy before detail: the big masses and long lines first, small turns later. \
        Check the negative shapes, the ground between and around the parts; fix one that is off \
        before going on.",
        "先层级后细节：先定大的体块和长线，再画小的起伏。检查负形，也就是各部分之间和周围的空白；哪里不对先改，再往下画。";
    SHAPE_4 = "Then draw the outline with Draw and tap Compare: the picture's outline lies over \
        yours in red, the picture faint under it; In place shows where you placed it, Aligned \
        its shape alone. Correct in blue and compare again.",
        "然后用画图笔画出外形，点“叠图对比”：原图的外形轮廓以红色叠在你的线条上，下面淡淡的是原图；原位看位置，对齐只比形状。用修正笔（蓝色）改，再对比。";
}

/// The instructions for a family: the heading and its paragraphs, the last saying once what
/// Compare shows (what red, blue, ink, dashed and the rings mean): the result cards never do.
pub(crate) const fn instructions(family: Family) -> (Text, &'static [Text]) {
    use super::practice_colour as c;
    use super::practice_perspective as p;
    use super::practice_points as s;
    match family {
        Family::Line => (COPY_A_LINE, &[LINE_1, LINE_2, LINE_3, LINE_4]),
        Family::Combined => (COPY_COMBINED, &[GROUP_1, GROUP_2, GROUP_3, GROUP_4]),
        Family::Shape => (COPY_A_SHAPE, &[SHAPE_1, SHAPE_2, SHAPE_3, SHAPE_4]),
        Family::Box => (
            p::COMPLETE_THE_BOX,
            &[p::BOX_1, p::BOX_2, p::BOX_3, p::BOX_4],
        ),
        Family::FreeBoxes => (
            p::FREE_BOXES_EXERCISE,
            &[p::FREE_1, p::FREE_2, p::FREE_3, p::FREE_4],
        ),
        Family::Ellipse => (
            p::DRAW_AN_ELLIPSE,
            &[p::ELLIPSE_1, p::ELLIPSE_2, p::ELLIPSE_3, p::ELLIPSE_4],
        ),
        Family::Point => s::estimate_steps(Estimate::Point),
        Family::Proportion => s::estimate_steps(Estimate::Proportion),
        Family::Angle => s::estimate_steps(Estimate::Angle),
        Family::Centre => s::trick_steps(Trick::Centre),
        Family::Halves => s::trick_steps(Trick::Halves),
        Family::Thirds => s::trick_steps(Trick::Thirds),
        Family::Step => s::trick_steps(Trick::Step),
        Family::Mirror => s::trick_steps(Trick::Mirror),
        Family::Colour => (
            MATCH_A_COLOUR,
            &[COLOUR_1, COLOUR_2, COLOUR_3, COLOUR_4, COLOUR_5],
        ),
        Family::Areas => (
            c::AREAS_EXERCISE,
            &[c::AREAS_1, c::AREAS_2, c::AREAS_3, c::AREAS_4],
        ),
        Family::Scale => (
            c::SCALE_EXERCISE,
            &[c::SCALE_1, c::SCALE_2, c::SCALE_3, c::SCALE_4],
        ),
        Family::Study => (
            c::STUDY_EXERCISE,
            &[c::STUDY_1, c::STUDY_2, c::STUDY_3, c::STUDY_4],
        ),
    }
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
