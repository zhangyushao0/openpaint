//! Practice in the gallery: the Today card, the practice options' sections and what the options
//! tell in the gallery's notice pill.

use openpaint_app::FileProblem;
use openpaint_practice_app::OptionsNotice;

use super::{FILE_UNREADABLE, Language, Text};

texts! { PRACTICE_TODAY;
    TODAY = "Today", "今天";
    PRACTICE = "Practice", "练习";
    START_PRACTICE = "Start practice", "开始练习";
    CONTINUE_PRACTICE = "Continue practice", "继续练习";
    LINES_AND_COLOURS = "Lines and colours", "线条与色彩";
    SINGLE_SHAPES = "Single shapes", "单体图形";
    PRACTICE_OPTIONS = "Practice options", "练习选项";
    START_NEXT_STAGE = "Start the next stage", "开始下一阶段";
    START_A_SESSION = "Start a session", "开始一次练习";
    DRILL_NEEDS_PACK = "Shapes copy a practice pack's pictures: import one under Practice packs.",
        "单体图形临摹的是素材包里的图片：请在“素材包”下导入一个。";
    PRACTICE_PACKS = "Practice packs", "素材包";
    MY_PICTURES = "My pictures", "我的图片";
    IMPORT_PACK = "Import practice pack…", "导入素材包…";
    ADD_PICTURE = "Add my picture…", "添加我的图片…";
    NO_PACKS = "No pack yet: single shapes copy a pack's pictures.",
        "还没有素材包：单体图形要临摹素材包里的图片。";
    PACKS_NOTE = "Your own pictures stay on this tablet; one comes up in each stage 2 or 3 \
        session that reaches its tier.",
        "你的图片只存在这台平板上。第 2、3 阶段练到它的难度后，每次练习会用到其中一张。";
    SHAPES_NEED_PACK = "Shapes need a practice pack: see the options.",
        "单体图形需要素材包：见练习选项。";
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

/// The Today card's stage line: "Stage 1 · Lines and colours".
pub(crate) fn stage(language: Language, stage: u8) -> String {
    let topic = stage_topic(stage).get(language);
    match language {
        Language::English => format!("Stage {stage} · {topic}"),
        Language::Chinese => format!("第 {stage} 阶段 · {topic}"),
    }
}

/// What a stage the app holds practises: "Lines and colours", "Single shapes", "Points",
/// "Simplifying", "Key shifts".
pub(crate) const fn stage_topic(stage: u8) -> Text {
    match stage {
        2 => SINGLE_SHAPES,
        3 => super::POINTS_STAGE,
        4 => super::SIMPLIFYING_STAGE,
        5 => super::KEY_SHIFTS_STAGE,
        _ => LINES_AND_COLOURS,
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
