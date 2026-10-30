//! Week 2's colour work on a practice sheet (colour combinations, value scales) and the long
//! colour copy: the bar's names, the painting pens, the cards' words (the How cards are
//! `practice_demo_colour.rs`) and the practice options' colour copies. In V大's terms: 色彩组合,
//! 明度推移, 色彩临摹, 大作业, 叠图对比, 原位, 色相, 饱和度, 明度, 弱对比.

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

/// A value scale compared (S97): how many of its cells are uneven.
pub(crate) fn cells_uneven(language: Language, cells: usize) -> String {
    match language {
        Language::English if cells == 1 => "1 cell uneven.".to_owned(),
        Language::English => format!("{cells} cells uneven."),
        Language::Chinese => format!("{cells} 格不均匀。"),
    }
}

/// A colour combination's area on the card: "Area 3".
pub(crate) fn area(language: Language, number: usize) -> String {
    match language {
        Language::English => format!("Area {number}"),
        Language::Chinese => format!("色块 {number}"),
    }
}
