//! A practice sheet: the bar, the exercise's own tools, the card (instructions after V大's method,
//! in our own words; comparisons in his terms: 叠图对比, 原位, 对齐, 更红/更黄, 更亮/更暗,
//! 更纯/更灰; 组合线条, 热身) and the summary.

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
    SHOWS_SHAPE_IN_PLACE = "In red: the picture's outline where it lies in its frame, the picture \
        faint under it. Where you placed it shows too.",
        "红色是原图的外形轮廓，在它框里的原位，下面淡淡的是原图。也能看出你画的位置准不准。";
    SHOWS_SHAPE_ALIGNED = "In red: the outline moved and sized onto your lines, never turned, the \
        picture faint with it. Its shape alone shows.",
        "红色是外形轮廓，已移动并缩放到你的线条上，不旋转，原图也淡淡地跟着。只比形状。";
    SHOWS_PICTURE_ONLY = "This picture has no outline: it lies faint over your copy where it lies \
        in its frame. Aligned needs an outline.",
        "这张图没有外形轮廓：原图淡淡地叠在你的临摹上，在它框里的原位。对齐需要轮廓。";
    ALIGNING = "Aligning…", "正在对齐…";
    SHOWS_ALIGNED = "In red: the original moved and sized onto your line, never turned. Its \
        shape alone shows, wherever you placed it.",
        "红色是原线条，已移动并缩放到你的线条上，不旋转。只比形状，不论你画在哪里。";
    SHOWS_IN_PLACE = "In red: the original where it lies in its frame. Where you placed it shows \
        too.", "红色是原线条，在它框里的原位。也能看出你画的位置准不准。";
    CORRECT_IN_BLUE = "Draw over your line in blue to correct it, then tap Compare again.",
        "用蓝色在你的线条上修正，然后再点一次“叠图对比”。";
    CORRECT_OVER = "Draw over your line to correct it, then tap Compare again.",
        "在你的线条上修正，然后再点一次“叠图对比”。";
    SHOWS_ALIGNED_GROUP = "In red: the whole group moved and sized onto your lines as one shape, \
        never turned. The shape alone shows, wherever you placed it.",
        "红色是整组原线条，作为一个形状移动并缩放到你的线条上，不旋转。只比形状，不论你画在哪里。";
    SHOWS_IN_PLACE_GROUP = "In red: the original lines where they lie in their frame. Where you \
        placed them shows too.", "红色是原线条，在它们框里的原位。也能看出你画的位置准不准。";
    CORRECT_GROUP_IN_BLUE = "Draw over your lines in blue to correct them, then tap Compare \
        again.", "用蓝色在你的线条上修正，然后再点一次“叠图对比”。";
    CORRECT_GROUP_OVER = "Draw over your lines to correct them, then tap Compare again.",
        "在你的线条上修正，然后再点一次“叠图对比”。";
    SAME_COLOUR = "The same hue, purity and lightness.", "色相、纯度和明度都一样。";
    WHEEL_NOTE = "Yours in ink, the original in red, the arrows from yours to it. ΔE: Oklab × \
        100; about 2 is just noticeable.",
        "墨色是你的颜色，红色是原色，箭头从你的指向原色。ΔE：Oklab × 100，约 2 时刚能看出差别。";
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
    LINE_4 = "Then tap Compare: the original lies over your line in red, in place or aligned to \
        it. Correct in blue and compare again, as often as you like.",
        "然后点“叠图对比”：原线条以红色叠在你的线条上，可选原位或对齐。用修正笔改，再对比，次数不限。";
    GROUP_1 = "Copy the group of lines into the empty frame beside it.",
        "把这组线条临摹到旁边的空白框里。";
    GROUP_2 = "See the whole group as one shape first: its outer ends and corners, where lines \
        meet or cross. Fix those key points in the frame before anything else, lightly with \
        Sketch.",
        "先把整组线条看成一个形状：外围的端点和转角，线条相接或交叉的地方。先用草稿笔轻轻定下这些关键点在框里的位置。";
    GROUP_3 = "Then the inside: each line between its points, its bends found as for a single \
        line, in sure strokes with Draw. Don't erase; draw the better line over it.",
        "再画里面：每条线连接它的关键点，弯曲处像单根线条一样找。用画图笔果断画出，不要擦，画错就在上面画一条更准的。";
    GROUP_4 = "Compare, don't guess: tap Compare. In place shows where the group lies; Aligned \
        lays the whole group onto yours as one shape. Correct in blue and compare again.",
        "要对比，别凭感觉画：点“叠图对比”。原位看位置，对齐把整组线条当成一个形状叠到你的线条上。用修正笔改，再对比。";
    COLOUR_1 = "Fill the empty frame with the colour of the swatch beside it.",
        "用旁边色块的颜色填满空白框。";
    COLOUR_2 = "Name it first: its hue, how pure or grey, how light or dark. Then pick it once by \
        eye on the wheel and tap Fill frame.",
        "先说出它的色相、纯度和明暗：偏纯还是偏灰，偏亮还是偏暗。再凭眼睛在色轮上选一次，点“填满方框”。";
    COLOUR_3 = "Compare by eye: redder or yellower? Lighter or darker? Purer or greyer? Pick and \
        fill again two or three times.",
        "用眼睛比较：更红还是更黄？更亮还是更暗？更纯还是更灰？再选色填充两三次。";
    COLOUR_4 = "Then tap Compare: the wheel shows where your colour lies against the original. \
        The swatch can't be picked with the eyedropper before.",
        "然后点“叠图对比”：色轮上会显示你的颜色和原色的位置。对比之前，吸管吸不到色块。";
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
    SHAPE_4 = "Then draw the outline with Draw and tap Compare: In place shows where you placed \
        it, Aligned its shape alone. Correct in blue and compare again.",
        "然后用画图笔画出外形，点“叠图对比”：原位看位置，对齐只比形状。用修正笔改，再对比。";
}

/// The instructions for a family: the heading and four paragraphs.
pub(crate) const fn instructions(family: Family) -> (Text, [Text; 4]) {
    use super::practice_colour as c;
    use super::practice_perspective as p;
    match family {
        Family::Line => (COPY_A_LINE, [LINE_1, LINE_2, LINE_3, LINE_4]),
        Family::Combined => (COPY_COMBINED, [GROUP_1, GROUP_2, GROUP_3, GROUP_4]),
        Family::Shape => (COPY_A_SHAPE, [SHAPE_1, SHAPE_2, SHAPE_3, SHAPE_4]),
        Family::Box => (
            p::COMPLETE_THE_BOX,
            [p::BOX_1, p::BOX_2, p::BOX_3, p::BOX_4],
        ),
        Family::FreeBoxes => (
            p::FREE_BOXES_EXERCISE,
            [p::FREE_1, p::FREE_2, p::FREE_3, p::FREE_4],
        ),
        Family::Ellipse => (
            p::DRAW_AN_ELLIPSE,
            [p::ELLIPSE_1, p::ELLIPSE_2, p::ELLIPSE_3, p::ELLIPSE_4],
        ),
        Family::Colour => (MATCH_A_COLOUR, [COLOUR_1, COLOUR_2, COLOUR_3, COLOUR_4]),
        Family::Areas => (
            c::AREAS_EXERCISE,
            [c::AREAS_1, c::AREAS_2, c::AREAS_3, c::AREAS_4],
        ),
        Family::Scale => (
            c::SCALE_EXERCISE,
            [c::SCALE_1, c::SCALE_2, c::SCALE_3, c::SCALE_4],
        ),
        Family::Study => (
            c::STUDY_EXERCISE,
            [c::STUDY_1, c::STUDY_2, c::STUDY_3, c::STUDY_4],
        ),
    }
}

/// What a shape's overlay shows (`aligned` with an outline, else in place; `outline` whether
/// the picture has one), and how to go on.
pub(crate) const fn compared_shape(aligned: bool, outline: bool, blue: bool) -> [Text; 2] {
    let shows = match (outline, aligned) {
        (false, _) => SHOWS_PICTURE_ONLY,
        (true, true) => SHOWS_SHAPE_ALIGNED,
        (true, false) => SHOWS_SHAPE_IN_PLACE,
    };
    let next = if blue {
        CORRECT_GROUP_IN_BLUE
    } else {
        CORRECT_GROUP_OVER
    };
    [shows, next]
}

/// What a drawn exercise's overlay shows (`aligned`, else in place), and how to go on (in blue,
/// else with the pen in hand), for one line or a group.
pub(crate) const fn compared_lines(group: bool, aligned: bool, blue: bool) -> [Text; 2] {
    let shows = match (group, aligned) {
        (false, true) => SHOWS_ALIGNED,
        (false, false) => SHOWS_IN_PLACE,
        (true, true) => SHOWS_ALIGNED_GROUP,
        (true, false) => SHOWS_IN_PLACE_GROUP,
    };
    let next = match (group, blue) {
        (false, true) => CORRECT_IN_BLUE,
        (false, false) => CORRECT_OVER,
        (true, true) => CORRECT_GROUP_IN_BLUE,
        (true, false) => CORRECT_GROUP_OVER,
    };
    [shows, next]
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
        Axis::Saturation => super::SATURATION,
        Axis::Value => super::VALUE,
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
        Metric::FirstSaturation | Metric::FinalSaturation => super::SATURATION,
        Metric::FirstValue | Metric::FinalValue => super::VALUE,
        Metric::Corrections => CORRECTIONS,
        Metric::AreasDelta => super::AREAS_DELTA,
        Metric::AreasWeak => super::WEAK_CONTRASTS,
        Metric::AreasMadeStrong => super::WEAK_MADE_STRONG,
        Metric::ScaleOff => super::OFF_EVEN,
        Metric::ScaleUneven => super::UNEVEN_CELLS,
        Metric::ScaleReversed => super::OUT_OF_ORDER,
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
    match (language, family) {
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

/// How a colour differs, its parts already in words ("12° yellower"): "Yours is 12° yellower,
/// 8 % greyer.", or the same hue and all where there are none.
pub(crate) fn yours_is(language: Language, parts: &[String]) -> String {
    if parts.is_empty() {
        return SAME_COLOUR.get(language).to_owned();
    }
    match language {
        Language::English => format!("Yours is {}.", parts.join(", ")),
        Language::Chinese => format!("你的颜色：{}。", parts.join("，")),
    }
}

/// One part of a difference by an amount: "12° yellower", "8 % greyer"; Chinese puts the word
/// first ("更黄 12°").
pub(crate) fn by(language: Language, amount: &str, way: Text) -> String {
    match language {
        Language::English => format!("{amount} {}", way.get(language)),
        Language::Chinese => format!("{} {amount}", way.get(language)),
    }
}

/// Another hue altogether: "orange where it is red".
pub(crate) fn instead(language: Language, mine: Text, theirs: Text) -> String {
    match language {
        Language::English => format!(
            "{} where it is {}",
            mine.get(language),
            theirs.get(language)
        ),
        Language::Chinese => format!("{}（原色是{}）", mine.get(language), theirs.get(language)),
    }
}

/// What the first Compare kept of a colour.
pub(crate) fn kept_from_first(
    language: Language,
    (first, last): (&str, &str),
    corrections: f64,
) -> String {
    match language {
        Language::English => {
            let plural = if corrections == 1.0 { "" } else { "s" };
            format!(
                "Kept from your first Compare: ΔE {first} at the first pick, {last} at the \
                 last, after {corrections:.0} correction{plural}."
            )
        }
        Language::Chinese => format!(
            "首次对比时记下：第一次选色 ΔE {first}，最后一次 {last}，修正 {corrections:.0} 次。"
        ),
    }
}

/// What the summary's radar shows.
pub(crate) fn radar_note(language: Language, stage: u8) -> String {
    match language {
        Language::English => format!(
            "The shape: your colour judgement, the rim exact. Hue falls to the centre at 30° off, \
             saturation and value at 25 %, dark and low-saturation colours at ΔE 10; first to \
             final: how much of the first pick's error your corrections took away. Dashed: your \
             earlier sessions in stage {stage}."
        ),
        Language::Chinese => format!(
            "图形是你的颜色判断力，外圈为完全准确。色相差 30°、饱和度和明度差 25 %、暗色和低饱和色 \
             ΔE 差 10 时落到中心；首次 → 最终：你的修正消除了首次选色误差的多少。虚线：你在第 \
             {stage} 阶段此前的练习。"
        ),
    }
}
