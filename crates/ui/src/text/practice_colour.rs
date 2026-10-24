//! Week 2's colour work on a practice sheet (colour combinations, value scales) and the long
//! colour copy: the bar's names, the painting pens, the cards' words and the instructions (after
//! V大's method, in our own words: the overall tone first and the frame covered, then the middle
//! tones by colour with a big brush, then the hierarchy with a smaller one; weak contrasts kept
//! weak; the last paragraph says once what Compare shows), and the practice options' colour
//! copies. In V大's terms: 色彩组合, 明度推移, 色彩临摹, 大作业, 叠图对比, 原位, 色相, 饱和度,
//! 明度, 弱对比.

use super::Language;

texts! { PRACTICE_COLOUR;
    AREAS_EXERCISE = "Colour combination", "色彩组合";
    SCALE_EXERCISE = "Value scale", "明度推移";
    STUDY_EXERCISE = "Colour copy", "色彩临摹";
    BIG = "Big", "大笔";
    SMALL = "Small", "小笔";
    SOFT = "Soft", "柔边笔";
    FILL_CELL = "Fill cell", "填满格子";
    ORIGINAL_SHOWN = "Original shown", "原图显示";
    READING_COPY = "Reading your copy…", "正在读取你的临摹…";
    NOT_PAINTED = "This area is not painted yet.", "这个色块还没有画。";
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
    AREAS_1 = "Copy the colours into the empty frame by eye (the original can't be picked before \
        Compare, your copy can). Squint for the overall tone and cover the frame with it first: \
        Fill frame.",
        "凭眼睛把颜色临摹到空白框里（叠图对比之前不能吸原图，可以吸你自己的）。眯眼看整体色调，先用“填满方框”把框铺满。";
    AREAS_2 = "Then the big areas by colour with Big, each judged against its neighbours: redder \
        or yellower, lighter or darker, purer or greyer.",
        "再用大笔按颜色分出大块，每块都和旁边的比：更红还是更黄，更亮还是更暗，更纯还是更灰。";
    AREAS_3 = "Then the smaller areas and the edges with Small and Soft, soft where the original \
        is. Keep weak contrasts weak: most copies make them strong.",
        "最后用小笔和柔边笔画小块和边缘，原图柔和的地方就画柔。弱对比要保持弱：大多数人会把它画强。";
    AREAS_4 = "Compare lays the original over your copy, the slider setting how strongly; each \
        chip, the original's colour over yours, shows its pair on the wheel and in the table; a \
        weak contrast made strong is red. Then the eyedropper compares any point.",
        "叠图对比把原图叠在你的临摹上，滑块调节强弱；每个色片上为原色、下为你的颜色，点它在色轮和表里比较；画强了的弱对比标红。之后用吸管点任意位置比较。";
    SCALE_1 = "Fill the cells between the dark and the light end in even steps of lightness as \
        your eye sees it: each step as far from the one before as from the one after.",
        "在暗端和亮端之间的格子里，按眼睛看到的明度均匀地一级一级填色：每一级和前后两级的差距都相同。";
    SCALE_2 = "Keep the hue and the purity: move only up and down in the wheel's square, and \
        judge each step by eye. The ends can be picked.",
        "保持色相和饱和度不变，只在色轮的方块里上下移动，每一级都用眼睛判断。两端的颜色可以吸取。";
    SCALE_3 = "Pick a value and tap Fill cell; the next cell comes in hand. The pen landing in a \
        cell takes that one, to paint or fill again.",
        "选好明度，点“填满格子”，下一格就接着选中。笔落在哪一格就选中哪一格，可以重画或重填。";
    SCALE_4 = "Then tap Compare: the even scale shows in the original's frame; under your row \
        each cell's lightness as the eye sees it as a bar, its even step a red tick, an uneven \
        cell's bar outlined red. Fill a cell again to correct it and compare again.",
        "然后点“叠图对比”：原图框里显示均匀的推移；你的格子下方每格人眼看到的明度画成竖条，红线是均匀的那一级，不均匀的格子竖条描红。重填某一格来修正，再对比。";
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
        place, as strongly as the slider sets; then the eyedropper on any point shows both \
        colours there on the wheel and in the table.",
        "叠图对比之前不能吸原图的颜色。叠图对比把原图叠在你的临摹上的原位，显示多少由滑块调节；之后用吸管点任意位置，两边该处的颜色会显示在色轮和表里。";
}

/// A colour combination's area on the card: "Area 3".
pub(crate) fn area(language: Language, number: usize) -> String {
    match language {
        Language::English => format!("Area {number}"),
        Language::Chinese => format!("色块 {number}"),
    }
}
