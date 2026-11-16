//! The practice options (S80, S83): the packs kept and importing one, the tiers in the author's
//! words (入门, 简单, 中等, 难), choosing a colour copy's picture, a copy's page (open, rename,
//! remove), My pictures (each picture's tier, its removal and what that leaves), adding one at a
//! tier, and what the options tell in the home's notice pill.

use openpaint_app::FileProblem;
use openpaint_practice::Tier;
use openpaint_practice_app::OptionsNotice;

use super::{FILE_UNREADABLE, Language, Text};

texts! { PRACTICE_PACKS_TABLE;
    PRACTICE_PACKS = "Practice packs", "素材包";
    MY_PICTURES = "My pictures", "我的图片";
    IMPORT_PACK = "Import practice pack…", "导入素材包…";
    ADD_PICTURE = "Add my picture…", "添加我的图片…";
    NO_PACKS = "No pack yet.", "还没有素材包。";
}

texts! { PRACTICE_NOTICES;
    PACK_IMPORTED = "Practice pack imported: single shapes.", "素材包已导入：单体图形。";
    COMPOSITIONS_IMPORTED = "Practice pack imported: compositions.", "素材包已导入：构图。";
    COMPOSITIONS_AND_CROPS_IMPORTED = "Practice pack imported: compositions and colour crops.",
        "素材包已导入：构图和色彩组合的局部。";
    CROPS_IMPORTED = "Practice pack imported: colour crops.", "素材包已导入：色彩组合的局部。";
    PACK_DAMAGED = "Couldn't import: the file is not a practice pack, or it is damaged.",
        "无法导入：这不是素材包，或文件已损坏。";
    PACK_OTHER_FORMAT = "Couldn't import: the pack is from another version of OpenPaint.",
        "无法导入：这个素材包来自其他版本的 OpenPaint。";
    PICTURE_ADDED = "Added to My pictures, with its outline.", "已添加到“我的图片”，有外轮廓。";
    PICTURE_ADDED_PLAIN = "Added to My pictures, without an outline.",
        "已添加到“我的图片”，没有外轮廓。";
    PICTURE_REFUSED = "Couldn't add the picture: not a PNG or JPEG.",
        "无法添加图片：不是 PNG 或 JPEG。";
}

/// What the practice options' notice says.
pub(crate) const fn options_notice(notice: OptionsNotice) -> Text {
    match notice {
        OptionsNotice::PackImported {
            compositions: false,
            crops: false,
        } => PACK_IMPORTED,
        OptionsNotice::PackImported {
            compositions: true,
            crops: false,
        } => COMPOSITIONS_IMPORTED,
        OptionsNotice::PackImported {
            compositions: true,
            crops: true,
        } => COMPOSITIONS_AND_CROPS_IMPORTED,
        OptionsNotice::PackImported {
            compositions: false,
            crops: true,
        } => CROPS_IMPORTED,
        OptionsNotice::PackRefused(FileProblem::OtherFormat) => PACK_OTHER_FORMAT,
        OptionsNotice::PackRefused(FileProblem::Unreadable) => FILE_UNREADABLE,
        OptionsNotice::PackRefused(FileProblem::NotOpenPaint | FileProblem::Damaged) => {
            PACK_DAMAGED
        }
        OptionsNotice::PictureAdded { outline: true } => PICTURE_ADDED,
        OptionsNotice::PictureAdded { outline: false } => PICTURE_ADDED_PLAIN,
        OptionsNotice::PictureRefused => PICTURE_REFUSED,
    }
}

/// A pack's line in the options: "Single shapes: 126 pictures, 60 with an outline"; a pack of
/// pictures never outlined (compositions) its pictures alone.
pub(crate) fn pack_line(
    language: Language,
    title: &str,
    (pictures, outlines): (usize, Option<usize>),
) -> String {
    match (language, outlines) {
        (Language::English, outlines) => {
            let plural = if pictures == 1 { "" } else { "s" };
            let line = format!("{title}: {pictures} picture{plural}");
            match outlines {
                Some(outlines) => format!("{line}, {outlines} with an outline"),
                None => line,
            }
        }
        (Language::Chinese, Some(outlines)) => {
            format!("{title}：{pictures} 张图片，{outlines} 张有轮廓")
        }
        (Language::Chinese, None) => format!("{title}：{pictures} 张图片"),
    }
}

texts! { PRACTICE_OPTIONS_TABLE;
    ENTRY = "Entry", "入门";
    SIMPLE = "Simple", "简单";
    MEDIUM = "Medium", "中等";
    HARD = "Hard", "难";
    ENTRY_HINT = "Bold, closed outlines", "轮廓醒目、闭合";
    SIMPLE_HINT = "Clean outlines, plain grounds", "轮廓干净，背景简单";
    MEDIUM_HINT = "Longer, softer outlines", "轮廓更长、更柔和";
    HARD_HINT = "Many parts and overlaps", "部件多，互相遮挡";
    NEW_COPY = "New copy…", "新建临摹…";
    CHOOSE_PICTURE = "Choose a picture", "选一张图片";
    NO_PICTURES_HERE = "No pictures here.", "这里没有图片。";
    COPIED_BEFORE = "Copied before", "已临摹过";
    COPY_OPTIONS = "Copy options", "临摹选项";
    RENAME_COPY = "Rename…", "重命名…";
    REMOVE = "Delete…", "删除…";
    PICTURE_REMOVED = "Picture deleted", "图片已删除";
    COPY_GOES = "Your painting on it goes for good.", "你画的内容会永久删除。";
    ADD_MY_PICTURE = "Add my picture", "添加我的图片";
    HOW_HARD = "How hard to copy", "临摹难度";
    WITH_OUTLINE = "With an outline", "有外轮廓";
    WITHOUT_OUTLINE = "No outline", "没有外轮廓";
    REMOVE_PICTURE = "Delete picture", "删除图片";
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

/// What removing a picture of one's own leaves, by how many lessons, colour copies and drawings
/// from life (S151) used it.
pub(crate) fn picture_goes(
    language: Language,
    (lessons, copies, drawings): (u32, u32, u32),
) -> String {
    let gone = match language {
        Language::English => "It goes from this tablet for good.",
        Language::Chinese => "它会从这台平板上永久删除。",
    };
    if lessons == 0 && copies == 0 && drawings == 0 {
        return match language {
            Language::English => format!("{gone} No lesson or copy used it."),
            Language::Chinese => format!("{gone}没有课或临摹用过它。"),
        };
    }
    let users = match language {
        Language::English => {
            let plural = |n: u32, one: &str, many: &str| {
                (n > 0).then(|| format!("{n} {}", if n == 1 { one } else { many }))
            };
            let parts: Vec<String> = [
                plural(lessons, "lesson", "lessons"),
                plural(copies, "colour copy", "colour copies"),
                plural(drawings, "drawing from life", "drawings from life"),
            ]
            .into_iter()
            .flatten()
            .collect();
            parts.join(" and ")
        }
        Language::Chinese => {
            let parts: Vec<String> = [
                (lessons > 0).then(|| format!("{lessons} 课")),
                (copies > 0).then(|| format!("{copies} 幅色彩临摹")),
                (drawings > 0).then(|| format!("{drawings} 幅写生")),
            ]
            .into_iter()
            .flatten()
            .collect();
            parts.join("和 ")
        }
    };
    match language {
        Language::English => format!("{gone} {users} used it."),
        Language::Chinese => format!("{gone}有 {users}用过它。"),
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
