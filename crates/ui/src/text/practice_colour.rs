//! Week 2's colour work on a practice sheet (colour combinations, value scales) and the long
//! colour copy: the bar's names, the painting pens, the cards' words and the instructions (after
//! V大's method, in our own words: the overall tone first and the frame covered, then the middle
//! tones by colour with a big brush, then the hierarchy with a smaller one; weak contrasts kept
//! weak), and the practice options' colour copies. In V大's terms: 色彩组合, 明度推移, 色彩临摹,
//! 大作业, 叠图对比, 原位, 色相, 饱和度, 明度, 弱对比.

use super::{Language, Text};

texts! { PRACTICE_COLOUR;
    AREAS_EXERCISE = "Colour combination", "色彩组合";
    SCALE_EXERCISE = "Value scale", "明度推移";
    STUDY_EXERCISE = "Colour copy", "色彩临摹";
    BIG = "Big", "大笔";
    SMALL = "Small", "小笔";
    SOFT = "Soft", "柔边笔";
    FILL_CELL = "Fill cell", "填满格子";
    ORIGINAL_SHOWN = "Original shown", "原图显示";
    SHOWS_AREAS = "The original lies over your copy in place; the slider sets how much shows. Each \
        area's colours below: the original's over yours.",
        "原图叠在你的临摹上的原位，滑块调节显示多少。下面是每个色块的颜色：上为原色，下为你的颜色。";
    READING_COPY = "Reading your copy…", "正在读取你的临摹…";
    NOT_PAINTED = "This area is not painted yet.", "这个色块还没有画。";
    WEAK_KEPT = "Weak contrasts: every one kept weak, as in the original.",
        "弱对比：都和原图一样保持了弱。";
    PROBE_HINT = "With the eyedropper, touch a point of either picture: both colours there show \
        on the wheel.", "用吸管点任一边的一个位置：两边该处的颜色会显示在色轮上。";
    SHOWS_STUDY = "The original lies over your copy in place; the slider sets how much shows.",
        "原图叠在你的临摹上的原位，滑块调节显示多少。";
    SCALE_NOTE = "Under the row: each cell's lightness as the eye sees it, as a bar, its even \
        step marked in red.", "格子下方：每格人眼看到的明度画成竖条，红线标出均匀的那一级。";
    SCALE_EVEN = "Every cell is a quarter of a step or less from even, in order.",
        "每一格都在均匀位置的四分之一级以内，顺序也对。";
    SCALE_NEXT = "Fill a cell again to correct it, then tap Compare again.",
        "重新填某一格来修正，然后再点一次“叠图对比”。";
    COMBINATIONS = "Combinations", "色彩组合";
    SCALES = "Value scales", "明度推移";
    AREAS_DELTA = "Areas ΔE", "色块 ΔE";
    WEAK_CONTRASTS = "Weak contrasts", "弱对比";
    WEAK_MADE_STRONG = "Weak made strong", "弱对比画强";
    OFF_EVEN = "Off even", "偏离均匀";
    UNEVEN_CELLS = "Uneven cells", "不均匀的格";
    OUT_OF_ORDER = "Out of order", "顺序反了";
    COLOUR_COPIES = "Colour copies (long projects)", "色彩临摹（大作业）";
    OPEN = "Open", "打开";
    COPIES_NOTE = "A long copy in colour, worked on whenever you like over 10 to 20 hours; it never \
        counts as a session.", "用颜色做的长时间临摹，总共约 10 到 20 小时，随时都可以画；不算在练习次数里。";
}

texts! { PRACTICE_COLOUR_STEPS;
    AREAS_1 = "Copy the picture's colours into the empty frame beside it, by eye. The original \
        can't be picked before Compare; your own copy can.",
        "凭眼睛把这张图的颜色临摹到旁边的空白框里。叠图对比之前不能吸原图的颜色，可以吸你自己画上的颜色。";
    AREAS_2 = "Squint: what is the overall tone? Pick it on the wheel and tap Fill frame to cover \
        the frame first.", "眯起眼睛：整体是什么色调？在色轮上选出它，点“填满方框”，先把框铺满。";
    AREAS_3 = "Then the middle tones: the big areas, by colour, with Big. Judge each against its \
        neighbours: redder or yellower, lighter or darker, purer or greyer.",
        "再画中间调：用大笔按颜色分出大块。每块都和旁边的比：更红还是更黄，更亮还是更暗，更纯还是更灰。";
    AREAS_4 = "Then the hierarchy: the smaller areas and the edges, soft where the original is \
        soft, with Small and Soft. Keep weak contrasts weak: most copies make them too strong.",
        "最后是层次：用小笔和柔边笔画小块和边缘，原图柔和的地方就画柔。弱对比要保持弱：大多数人会把它画强。";
    SCALE_1 = "Fill the cells between the dark and the light end in even steps of lightness as \
        your eye sees it: each step as far from the one before as from the one after.",
        "在暗端和亮端之间的格子里，按眼睛看到的明度均匀地一级一级填色：每一级和前后两级的差距都相同。";
    SCALE_2 = "Keep the hue and the purity: move only up and down in the wheel's square, and \
        judge each step by eye. The ends can be picked.",
        "保持色相和饱和度不变，只在色轮的方块里上下移动，每一级都用眼睛判断。两端的颜色可以吸取。";
    SCALE_3 = "Pick a value and tap Fill cell; the next cell comes in hand. The pen landing in a \
        cell takes that one, to paint or fill again.",
        "选好明度，点“填满格子”，下一格就接着选中。笔落在哪一格就选中哪一格，可以重画或重填。";
    SCALE_4 = "Then tap Compare: under the row each cell's lightness shows as a bar, its even \
        step in red.", "然后点“叠图对比”：格子下方每格的明度显示成竖条，红线是均匀的那一级。";
    STUDY_1 = "Copy the picture in colour into the empty frame beside it, over as many sittings as \
        you like: about 10 to 20 hours in all. It never counts as a session.",
        "把这张图用颜色临摹到旁边的空白框里，分多次画完，总共约 10 到 20 小时。它不算在练习次数里。";
    STUDY_2 = "Squint for the overall tone and cover the frame first with a big brush; then the \
        middle tones, by colour; then the hierarchy and the detail, the brush smaller as you go.",
        "眯眼看整体色调，先用大笔铺满整个框；再按颜色分中间调；最后画层次和细节，笔越画越小。";
    STUDY_3 = "Bring every part to the same finish; zoom in and work a part as if it were the \
        whole picture. Keep weak contrasts weak.",
        "各处的完成度要一致；放大画局部时，把局部当成一张完整的画。弱对比要保持弱。";
    STUDY_4 = "The original can't be picked before Compare. Compare lays it over your copy in \
        place, as strongly as the slider sets; then the eyedropper compares any point on the wheel.",
        "叠图对比之前不能吸原图的颜色。叠图对比把原图叠在你的临摹上的原位，显示多少由滑块调节；之后用吸管点任意位置，两边的颜色会显示在色轮上。";
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

/// A colour combination's area on the card: "Area 3".
pub(crate) fn area(language: Language, number: usize) -> String {
    match language {
        Language::English => format!("Area {number}"),
        Language::Chinese => format!("色块 {number}"),
    }
}

/// How a copy kept the weak contrasts: "Weak contrasts: 1 of 3 made strong (red on your copy).
/// Keep them as close as the original's."
pub(crate) fn weak_made_strong(language: Language, (strong, weak): (usize, usize)) -> String {
    match language {
        Language::English => format!(
            "Weak contrasts: {strong} of {weak} made strong (red on your copy). Keep them as \
             close as the original's."
        ),
        Language::Chinese => {
            format!("弱对比：{weak} 处中有 {strong} 处画强了（你的临摹上标红）。要和原图一样接近。")
        }
    }
}

/// How a value scale reads: "Even within a quarter step: 3 of 5 cells."
pub(crate) fn scale_even(language: Language, (even, filled): (usize, usize)) -> String {
    match language {
        Language::English => format!("Even within a quarter step: {even} of {filled} cells."),
        Language::Chinese => format!("在均匀位置四分之一级以内：{filled} 格中有 {even} 格。"),
    }
}

/// A value scale's cell off its even step: "Cell 3 too dark."
pub(crate) fn cell_off(language: Language, cell: usize, dark: bool) -> String {
    match (language, dark) {
        (Language::English, true) => format!("Cell {cell} too dark."),
        (Language::English, false) => format!("Cell {cell} too light."),
        (Language::Chinese, true) => format!("第 {cell} 格太暗。"),
        (Language::Chinese, false) => format!("第 {cell} 格太亮。"),
    }
}

/// Two cells of a value scale out of order: "Cells 3 and 4 out of order: each should be lighter
/// than the one before."
pub(crate) fn cells_reversed(language: Language, cell: usize) -> String {
    let before = cell - 1;
    match language {
        Language::English => format!(
            "Cells {before} and {cell} out of order: each should be lighter than the one before."
        ),
        Language::Chinese => format!("第 {before} 和第 {cell} 格顺序反了：每一格都应比前一格亮。"),
    }
}

/// A value scale's cell not filled: "Cell 2 not filled yet."
pub(crate) fn cell_empty(language: Language, cell: usize) -> String {
    match language {
        Language::English => format!("Cell {cell} not filled yet."),
        Language::Chinese => format!("第 {cell} 格还没有填。"),
    }
}
