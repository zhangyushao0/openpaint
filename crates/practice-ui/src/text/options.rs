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
    NO_PACKS = "No pack yet: single shapes copy a pack's pictures.",
        "还没有素材包：单体图形要临摹素材包里的图片。";
    PACKS_NOTE = "Your own pictures stay on this tablet; lessons of single shapes use them once \
        they reach their tier.",
        "你的图片只存在这台平板上。单体图形的课练到它的难度后会用到它们。";
}

texts! { PRACTICE_NOTICES;
    PACK_IMPORTED = "Practice pack imported: its pictures come up in single shapes.",
        "素材包已导入：单体图形会用到里面的图片。";
    COMPOSITIONS_IMPORTED = "Practice pack imported: for divisions, value masses and whole \
        copies.", "素材包已导入：用于画面分割、黑白灰和整图复制。";
    COMPOSITIONS_AND_CROPS_IMPORTED = "Practice pack imported: for divisions, value masses and \
        whole copies, and its crops for colour combinations.",
        "素材包已导入：用于画面分割、黑白灰和整图复制，截取的局部用于色彩组合。";
    CROPS_IMPORTED = "Practice pack imported: its crops come up in colour combinations.",
        "素材包已导入：截取的局部用于色彩组合。";
    PACK_DAMAGED = "Couldn't import: the file is not a practice pack, or it is damaged.",
        "无法导入：这不是素材包，或文件已损坏。";
    PACK_OTHER_FORMAT = "Couldn't import: the pack is from another version of OpenPaint.",
        "无法导入：这个素材包来自其他版本的 OpenPaint。";
    PICTURE_ADDED = "Added to My pictures. Its outline was found: Compare lays it over your copy.",
        "已添加到“我的图片”，并找到了它的外轮廓：叠图对比时会把轮廓叠在你的临摹上。";
    PICTURE_ADDED_PLAIN = "Added to My pictures. No outline was found for it: Compare lays the \
        picture itself over your copy, faint.",
        "已添加到“我的图片”，但没找到外轮廓：叠图对比时会把原图淡淡地叠在你的临摹上。";
    PICTURE_REFUSED = "Couldn't add the picture: only PNG and JPEG pictures are taken.",
        "无法添加图片：只支持 PNG 和 JPEG 图片。";
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
    CHOOSE_HINT = "Tap a picture to begin a long copy of it.", "轻点一张图片，开始一幅色彩临摹。";
    NO_PICTURES_HERE = "No pictures here.", "这里没有图片。";
    COPIED_BEFORE = "Copied before", "已临摹过";
    COPY_OPTIONS = "Copy options", "临摹选项";
    RENAME_COPY = "Rename…", "重命名…";
    REMOVE = "Delete…", "删除…";
    PICTURE_REMOVED = "Picture deleted", "图片已删除";
    COPY_GOES = "Your painting on it goes for good. Its picture stays.",
        "你画的内容会永久删除，原图会保留。";
    ADD_MY_PICTURE = "Add my picture", "添加我的图片";
    TIER_FIRST = "How hard is it to copy? Choose, then the picture.",
        "临摹它有多难？先选难度，再选图片。";
    HOW_HARD = "How hard to copy", "临摹难度";
    WITH_OUTLINE = "With an outline: Compare lays it over your copy.",
        "有外轮廓：叠图对比时会把轮廓叠在你的临摹上。";
    WITHOUT_OUTLINE = "No outline: Compare lays the picture itself over your copy, faint.",
        "没有外轮廓：叠图对比时会把原图淡淡地叠在你的临摹上。";
    REMOVE_PICTURE = "Delete picture", "删除图片";
    MINE_NOTE = "Your own pictures stay on this tablet. Lessons of single shapes use one once they \
        reach its tier.",
        "你的图片只存在这台平板上。单体图形的课练到它的难度后，每课会用到其中一张。";
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

/// What removing a picture of one's own leaves, by how many lessons and colour copies used it.
pub(crate) fn picture_goes(language: Language, (sessions, copies): (u32, u32)) -> String {
    let gone = match language {
        Language::English => "It goes from this tablet for good.",
        Language::Chinese => "它会从这台平板上永久删除。",
    };
    if sessions == 0 && copies == 0 {
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
                plural(sessions, "lesson", "lessons"),
                plural(copies, "colour copy", "colour copies"),
            ]
            .into_iter()
            .flatten()
            .collect();
            parts.join(" and ")
        }
        Language::Chinese => {
            let parts: Vec<String> = [
                (sessions > 0).then(|| format!("{sessions} 课")),
                (copies > 0).then(|| format!("{copies} 幅色彩临摹")),
            ]
            .into_iter()
            .flatten()
            .collect();
            parts.join("和 ")
        }
    };
    match language {
        Language::English => format!(
            "{gone} {users} used it: they keep your strokes, and where it lay they say “Picture \
             deleted”."
        ),
        Language::Chinese => {
            format!("{gone}有 {users}用过它：你画的都会保留，原来放图的地方会显示“图片已删除”。")
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
