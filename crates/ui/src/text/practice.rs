//! A practice sheet: the bar, the exercise's own tools, the card (instructions after V大's method,
//! in our own words; comparisons in his terms: 叠图对比, 原位, 对齐, 更红/更黄, 更亮/更暗,
//! 更纯/更灰) and the summary.

use openpaint_app::Pen;
use openpaint_practice::{Axis, Metric};

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
    COPY_A_LINE = "Copy a line", "单根线条";
    MATCH_A_COLOUR = "Match a colour", "单色";
    SHOWS_ALIGNED = "In red: the original moved and sized onto your line, never turned. Its \
        shape alone shows, wherever you placed it.",
        "红色是原线条，已移动并缩放到你的线条上，不旋转。只比形状，不论你画在哪里。";
    SHOWS_IN_PLACE = "In red: the original where it lies in its frame. Where you placed it shows \
        too.", "红色是原线条，在它框里的原位。也能看出你画的位置准不准。";
    CORRECT_IN_BLUE = "Draw over your line in blue to correct it, then tap Compare again.",
        "用蓝色在你的线条上修正，然后再点一次“叠图对比”。";
    CORRECT_OVER = "Draw over your line to correct it, then tap Compare again.",
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
}

texts! { PRACTICE_HUES;
    REDDER = "redder", "更红";
    YELLOWER = "yellower", "更黄";
    GREENER = "greener", "更绿";
    MORE_CYAN = "more cyan", "更青";
    BLUER = "bluer", "更蓝";
    MORE_MAGENTA = "more magenta", "更偏品红";
    HUE_RED = "red", "红色";
    HUE_ORANGE = "orange", "橙色";
    HUE_YELLOW = "yellow", "黄色";
    HUE_YELLOW_GREEN = "yellow-green", "黄绿色";
    HUE_GREEN = "green", "绿色";
    HUE_GREEN_CYAN = "green-cyan", "青绿色";
    HUE_CYAN = "cyan", "青色";
    HUE_AZURE = "azure", "天蓝色";
    HUE_BLUE = "blue", "蓝色";
    HUE_VIOLET = "violet", "紫色";
    HUE_MAGENTA = "magenta", "品红色";
    HUE_ROSE = "rose", "玫红色";
    GREY = "grey", "灰色";
    PURER = "purer", "更纯";
    GREYER = "greyer", "更灰";
    LIGHTER = "lighter", "更亮";
    DARKER = "darker", "更暗";
}

/// The comparatives of the six hues 60 degrees apart, from red (`practice::hue_towards`).
pub(crate) const TOWARDS: [Text; 6] = [REDDER, YELLOWER, GREENER, MORE_CYAN, BLUER, MORE_MAGENTA];
/// The names of the twelve hues round the wheel, from red (`practice::hue_name`).
pub(crate) const HUE_NAMES: [Text; 12] = [
    HUE_RED,
    HUE_ORANGE,
    HUE_YELLOW,
    HUE_YELLOW_GREEN,
    HUE_GREEN,
    HUE_GREEN_CYAN,
    HUE_CYAN,
    HUE_AZURE,
    HUE_BLUE,
    HUE_VIOLET,
    HUE_MAGENTA,
    HUE_ROSE,
];

/// The instructions for lines (or colours): the heading and four paragraphs.
pub(crate) const fn instructions(lines: bool) -> (Text, [Text; 4]) {
    if lines {
        (COPY_A_LINE, [LINE_1, LINE_2, LINE_3, LINE_4])
    } else {
        (MATCH_A_COLOUR, [COLOUR_1, COLOUR_2, COLOUR_3, COLOUR_4])
    }
}

/// A line exercise's pens.
pub(crate) const fn pen(pen: Pen) -> Text {
    match pen {
        Pen::Sketch => SKETCH,
        Pen::Draw => DRAW,
        Pen::Correct => CORRECT,
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
    }
}

/// The exercise shown, as the bar names it: "Line 3 of 10".
pub(crate) fn exercise_of(language: Language, line: bool, (place, of): (usize, usize)) -> String {
    match (language, line) {
        (Language::English, true) => format!("Line {place} of {of}"),
        (Language::English, false) => format!("Colour {place} of {of}"),
        (Language::Chinese, true) => format!("线条 {place}/{of}"),
        (Language::Chinese, false) => format!("颜色 {place}/{of}"),
    }
}

/// A compared exercise's heading: "Line 3".
pub(crate) fn exercise(language: Language, line: bool, place: usize) -> String {
    match (language, line) {
        (Language::English, true) => format!("Line {place}"),
        (Language::English, false) => format!("Colour {place}"),
        (Language::Chinese, true) => format!("线条 {place}"),
        (Language::Chinese, false) => format!("颜色 {place}"),
    }
}

/// A finished session's heading.
pub(crate) fn session_done(language: Language, number: u32) -> String {
    match language {
        Language::English => format!("Session {number} done"),
        Language::Chinese => format!("第 {number} 次练习完成"),
    }
}

/// The summary's line per kind: "Lines: 10 of 10 compared".
pub(crate) fn compared_of(
    language: Language,
    lines: bool,
    (checked, held): (usize, usize),
) -> String {
    match (language, lines) {
        (Language::English, true) => format!("Lines: {checked} of {held} compared"),
        (Language::English, false) => format!("Colours: {checked} of {held} compared"),
        (Language::Chinese, true) => format!("线条：已对比 {checked}/{held}"),
        (Language::Chinese, false) => format!("颜色：已对比 {checked}/{held}"),
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
