//! The gallery: its header, the cards, the Today card, the dialogs, New canvas, its notices and
//! the dates of kept versions.

use openpaint_app::{CanvasPreset, FileProblem, GalleryNotice};
use openpaint_practice::Date;

use super::{Language, Text};

texts! { GALLERY_TABLE;
    SEARCH = "Search", "搜索";
    NEW_ARTWORK = "New artwork", "新建作品";
    NO_ARTWORKS = "No artworks yet. Tap + to start one.", "还没有作品，轻点 + 新建一幅。";
    TODAY = "Today", "今天";
    PRACTICE = "Practice", "练习";
    START_PRACTICE = "Start practice", "开始练习";
    CONTINUE_PRACTICE = "Continue practice", "继续练习";
    OPEN_ARTWORK = "Open artwork", "打开作品";
    DELETE_ARTWORK = "Delete artwork", "删除作品";
    DUPLICATE_ARTWORK = "Duplicate artwork", "复制作品";
    ARTWORK_OPTIONS = "Artwork options", "作品选项";
    RENAME_ARTWORK = "Rename artwork", "重命名作品";
    NO_PREVIEW = "No preview", "无预览";
    TITLE = "Title", "标题";
    CANNOT_BE_UNDONE = "This can't be undone. Its kept versions go with it.",
        "删除后无法撤销，它的历史版本也会一起删除。";
    SAVE_AS_FILE = "Save as OpenPaint file…", "存为 OpenPaint 文件…";
    VERSIONS = "Versions", "历史版本";
    KEPT_WHEN_LEFT = "Kept when you leave it after a change.", "改动作品后离开时，会保留一个版本。";
    OPENS_AS_COPY = "Each opens as a new copy.", "每个版本都会作为副本打开。";
    NEW_CANVAS = "New canvas", "新建画布";
    SCREEN_SIZE = "Screen size", "屏幕尺寸";
    CANVAS_SQUARE = "Square", "正方形";
    A4_PORTRAIT = "A4 portrait", "A4 竖向";
    A4_LANDSCAPE = "A4 landscape", "A4 横向";
    FROM_A_FILE = "From a file", "从文件";
    OPEN_FILE = "Open OpenPaint file…", "打开 OpenPaint 文件…";
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

texts! { GALLERY_NOTICES;
    LIST_FAILED = "Couldn't read the gallery.", "无法读取图库。";
    OTHER_LIBRARY = "These artworks are from another version of OpenPaint. They are left as they \
        are.", "这些作品来自其他版本的 OpenPaint，已原样保留。";
    OPEN_FAILED = "Couldn't open this artwork. It is kept as it was.", "无法打开此作品，已原样保留。";
    CREATE_FAILED = "Couldn't make a new artwork.", "无法新建作品。";
    CHANGE_FAILED = "Couldn't change the gallery.", "无法更改图库。";
    FILE_SAVED = "Saved as an OpenPaint file.", "已存为 OpenPaint 文件。";
    FILE_SAVE_FAILED = "Couldn't save the file.", "无法保存文件。";
    NOT_OPENPAINT = "Couldn't open the file: it is not an OpenPaint file.",
        "无法打开文件：这不是 OpenPaint 文件。";
    FILE_DAMAGED = "Couldn't open the file: it is damaged.", "无法打开文件：文件已损坏。";
    FILE_OTHER_FORMAT = "Couldn't open the file: it is from another version of OpenPaint.",
        "无法打开文件：它来自其他版本的 OpenPaint。";
    FILE_UNREADABLE = "Couldn't read the file.", "无法读取文件。";
    RESTORE_FAILED = "Couldn't restore this version.", "无法恢复此版本。";
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

/// What the gallery's notice says.
pub(crate) const fn gallery_notice(notice: GalleryNotice) -> Option<Text> {
    Some(match notice {
        GalleryNotice::PackImported {
            compositions: false,
            crops: false,
        } => PACK_IMPORTED,
        GalleryNotice::PackImported {
            compositions: true,
            crops: false,
        } => COMPOSITIONS_IMPORTED,
        GalleryNotice::PackImported {
            compositions: true,
            crops: true,
        } => COMPOSITIONS_AND_CROPS_IMPORTED,
        GalleryNotice::PackImported {
            compositions: false,
            crops: true,
        } => CROPS_IMPORTED,
        GalleryNotice::PackRefused(FileProblem::OtherFormat) => PACK_OTHER_FORMAT,
        GalleryNotice::PackRefused(FileProblem::Unreadable) => FILE_UNREADABLE,
        GalleryNotice::PackRefused(FileProblem::NotOpenPaint | FileProblem::Damaged) => {
            PACK_DAMAGED
        }
        GalleryNotice::PictureAdded { outline: true } => PICTURE_ADDED,
        GalleryNotice::PictureAdded { outline: false } => PICTURE_ADDED_PLAIN,
        GalleryNotice::PictureRefused => PICTURE_REFUSED,
        GalleryNotice::None => return None,
        GalleryNotice::ListFailed => LIST_FAILED,
        GalleryNotice::OtherLibrary => OTHER_LIBRARY,
        GalleryNotice::OpenFailed => OPEN_FAILED,
        GalleryNotice::CreateFailed => CREATE_FAILED,
        GalleryNotice::ChangeFailed => CHANGE_FAILED,
        GalleryNotice::FileSaved => FILE_SAVED,
        GalleryNotice::FileSaveFailed => FILE_SAVE_FAILED,
        GalleryNotice::FileRefused(FileProblem::NotOpenPaint) => NOT_OPENPAINT,
        GalleryNotice::FileRefused(FileProblem::Damaged) => FILE_DAMAGED,
        GalleryNotice::FileRefused(FileProblem::OtherFormat) => FILE_OTHER_FORMAT,
        GalleryNotice::FileRefused(FileProblem::Unreadable) => FILE_UNREADABLE,
        GalleryNotice::RestoreFailed => RESTORE_FAILED,
    })
}

/// A canvas size New canvas offers.
pub(crate) const fn preset(preset: CanvasPreset) -> Text {
    match preset {
        CanvasPreset::Screen => SCREEN_SIZE,
        CanvasPreset::Square => CANVAS_SQUARE,
        CanvasPreset::A4Portrait => A4_PORTRAIT,
        CanvasPreset::A4Landscape => A4_LANDSCAPE,
    }
}

/// What a search finds nothing for.
pub(crate) fn nothing_found(language: Language, search: &str) -> String {
    match language {
        Language::English => format!("No title holds “{search}”."),
        Language::Chinese => format!("没有标题含“{search}”的作品。"),
    }
}

/// The question before an artwork is deleted.
pub(crate) fn delete_artwork(language: Language, title: &str) -> String {
    match language {
        Language::English => format!("Delete “{title}”?"),
        Language::Chinese => format!("删除“{title}”？"),
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

/// A kept version's date and time of day: "5 Oct 2026 14:32", or "2026年10月5日 14:32";
/// `second` with the seconds.
pub(crate) fn kept_date(
    language: Language,
    Date { year, month, day }: Date,
    (hour, minute, second): (i64, i64, Option<i64>),
) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let time = match second {
        Some(second) => format!("{hour:02}:{minute:02}:{second:02}"),
        None => format!("{hour:02}:{minute:02}"),
    };
    let name = MONTHS[(month.clamp(1, 12) - 1) as usize];
    match language {
        Language::English => format!("{day} {name} {year} {time}"),
        Language::Chinese => format!("{year}年{month}月{day}日 {time}"),
    }
}
