//! Week 2's colour work on a practice sheet (colour combinations, value scales) and the long
//! colour copy: the painting pens, the cards' words (the kinds' names and How cards are
//! `practice/`) and the practice options' colour copies. In V大's terms: 色彩组合,
//! 明度推移, 色彩临摹, 大作业, 叠图对比, 原位, 色相, 饱和度, 明度, 弱对比.

use super::Language;

texts! { PRACTICE_COLOUR;
    BIG = "Big", "大笔";
    SMALL = "Small", "小笔";
    SOFT = "Soft", "柔边笔";
    FILL_CELL = "Fill cell", "填满格子";
    ORIGINAL_SHOWN = "Original shown", "原图浓淡";
    READING_COPY = "Reading your copy…", "正在读取你的临摹…";
    NOT_PAINTED = "This area is not painted yet.", "这个色块还没有画。";
    AREAS_DELTA = "Areas ΔE", "色块 ΔE";
    WEAK_CONTRASTS = "Weak contrasts", "弱对比";
    WEAK_MADE_STRONG = "Weak made strong", "画强的弱对比";
    OFF_EVEN = "Off even", "明度偏差";
    UNEVEN_CELLS = "Uneven cells", "不均匀的格子";
    OUT_OF_ORDER = "Out of order", "明暗颠倒";
    COLOUR_COPIES = "Colour copies (long projects)", "色彩临摹（大作业）";
    OPEN = "Open", "打开";
    COPIES_NOTE = "A long copy in colour, worked on whenever you like over 10 to 20 hours; it never \
        counts as a session.",
        "一幅慢慢画的色彩临摹，总共约 10 到 20 小时，什么时候画都行；不计入练习次数。";
    DRILL_NEEDS_CROPS = "Colour combinations and key shifts copy crops of real pictures: import \
        the Colour combinations or the Compositions pack under Practice packs.",
        "色彩组合和变调都要临摹真实图片的局部：请在“素材包”下导入“色彩组合”或“构图”素材包。";
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
