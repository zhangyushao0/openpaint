//! The practice options' pictures and copies (S83): the tiers in the author's words (入门, 简单,
//! 中等, 难), choosing a colour copy's picture, a copy's page (open, rename, remove), My pictures
//! (each picture's tier, its removal and what that leaves), and adding one at a tier.

use openpaint_practice::Tier;

use super::{Language, Text};

texts! { PRACTICE_OPTIONS_TABLE;
    ENTRY = "Entry", "入门";
    SIMPLE = "Simple", "简单";
    MEDIUM = "Medium", "中等";
    HARD = "Hard", "难";
    ENTRY_HINT = "Bold, closed outlines", "粗壮、封闭的外形";
    SIMPLE_HINT = "Clean outlines, plain grounds", "干净的外形，单色背景";
    MEDIUM_HINT = "Longer, softer outlines", "更长、更柔和的外形";
    HARD_HINT = "Many parts and overlaps", "部件多、互相重叠";
    NEW_COPY = "New copy…", "新建临摹…";
    CHOOSE_PICTURE = "Choose a picture", "选择一张图片";
    CHOOSE_HINT = "Tap a picture to begin a long copy of it.", "轻点一张图片，开始长时间临摹它。";
    NO_PICTURES_HERE = "No pictures here.", "这里没有图片。";
    COPIED_BEFORE = "Copied before", "临摹过";
    COPY_OPTIONS = "Copy options", "临摹选项";
    RENAME_COPY = "Rename…", "重命名…";
    REMOVE = "Delete…", "删除…";
    PICTURE_REMOVED = "Picture deleted", "图片已删除";
    COPY_GOES = "Your painting on it goes for good. Its picture stays.",
        "你在上面画的会被永久删除，原图保留。";
    ADD_MY_PICTURE = "Add my picture", "添加我的图片";
    TIER_FIRST = "How hard is it to copy? Choose, then the picture.",
        "临摹它有多难？先选难度，再选图片。";
    HOW_HARD = "How hard to copy", "临摹难度";
    WITH_OUTLINE = "With an outline: Compare lays it over your copy.",
        "有外形轮廓：叠图对比时会叠在你的临摹上。";
    WITHOUT_OUTLINE = "No outline: Compare lays the picture itself over your copy, faint.",
        "没有外形轮廓：叠图对比时会把原图淡淡地叠在你的临摹上。";
    REMOVE_PICTURE = "Delete picture", "删除图片";
    MINE_NOTE = "Your own pictures stay on this tablet. One comes up in each stage 2 session once \
        the sessions reach its tier.", "你自己的图片只保存在这台平板上。第 2 阶段的练习进行到它的难度后，每次练习会出现一张。";
    NO_PICTURE_OF_MINE = "No picture of your own yet.", "还没有你自己的图片。";
}

/// A tier's word.
pub(crate) const fn tier(tier: Tier) -> Text {
    match tier {
        Tier::Entry => ENTRY,
        Tier::Simple => SIMPLE,
        Tier::Medium => MEDIUM,
        Tier::Hard => HARD,
    }
}

/// What a tier holds, beside its word when a picture of one's own is added.
pub(crate) const fn tier_hint(tier: Tier) -> Text {
    match tier {
        Tier::Entry => ENTRY_HINT,
        Tier::Simple => SIMPLE_HINT,
        Tier::Medium => MEDIUM_HINT,
        Tier::Hard => HARD_HINT,
    }
}

/// The question before a colour copy or a picture of one's own is deleted: "Delete “Lamp”?".
pub(crate) fn remove_named(language: Language, name: &str) -> String {
    match language {
        Language::English => format!("Delete “{name}”?"),
        Language::Chinese => format!("删除“{name}”？"),
    }
}

/// What removing a picture of one's own leaves, by how many sessions and colour copies used it.
pub(crate) fn picture_goes(language: Language, (sessions, copies): (u32, u32)) -> String {
    let gone = match language {
        Language::English => "It goes from this tablet for good.",
        Language::Chinese => "它会从这台平板上永久删除。",
    };
    if sessions == 0 && copies == 0 {
        return match language {
            Language::English => format!("{gone} No session or copy used it."),
            Language::Chinese => format!("{gone}没有练习或临摹用过它。"),
        };
    }
    let users = match language {
        Language::English => {
            let plural = |n: u32, one: &str, many: &str| {
                (n > 0).then(|| format!("{n} {}", if n == 1 { one } else { many }))
            };
            let parts: Vec<String> = [
                plural(sessions, "session", "sessions"),
                plural(copies, "colour copy", "colour copies"),
            ]
            .into_iter()
            .flatten()
            .collect();
            parts.join(" and ")
        }
        Language::Chinese => {
            let parts: Vec<String> = [
                (sessions > 0).then(|| format!("{sessions} 次练习")),
                (copies > 0).then(|| format!("{copies} 份色彩临摹")),
            ]
            .into_iter()
            .flatten()
            .collect();
            parts.join("和")
        }
    };
    match language {
        Language::English => format!(
            "{gone} {users} used it: they keep your strokes, and where it lay they say “Picture \
             deleted”."
        ),
        Language::Chinese => {
            format!("{gone}有{users}用过它：你画的都会保留，原图的位置会显示“图片已删除”。")
        }
    }
}

/// A picture of one's own on its row: its tier, and whether it has an outline.
pub(crate) fn mine_line(language: Language, tier_word: &str, outline: bool) -> String {
    match (language, outline) {
        (Language::English, true) => format!("{tier_word} · outline"),
        (Language::English, false) => format!("{tier_word} · no outline"),
        (Language::Chinese, true) => format!("{tier_word} · 有轮廓"),
        (Language::Chinese, false) => format!("{tier_word} · 无轮廓"),
    }
}

/// My pictures' row in the options: "3 pictures".
pub(crate) fn pictures_count(language: Language, count: usize) -> String {
    match language {
        Language::English if count == 1 => "1 picture".to_owned(),
        Language::English => format!("{count} pictures"),
        Language::Chinese => format!("{count} 张图片"),
    }
}
