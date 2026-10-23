//! The gallery: its header, the cards, the Today card, the dialogs, New canvas, its notices and
//! the dates of kept versions.

use openpaint_app::{CanvasPreset, FileProblem, GalleryNotice};

use super::{Language, Text};

texts! { GALLERY_TABLE;
    SEARCH = "Search", "搜索";
    NEW_ARTWORK = "New artwork", "新建作品";
    NO_ARTWORKS = "No artworks yet. Tap + to start one.", "还没有作品。轻点 + 开始创作。";
    TODAY = "Today", "今日";
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
        "此操作无法撤销，保留的版本也会一并删除。";
    SAVE_AS_FILE = "Save as OpenPaint file…", "存为 OpenPaint 文件…";
    VERSIONS = "Versions", "版本";
    KEPT_WHEN_LEFT = "Kept when you leave it after a change.", "改动后离开作品时保留。";
    OPENS_AS_COPY = "Each opens as a new copy.", "每个版本都作为新副本打开。";
    NEW_CANVAS = "New canvas", "新建画布";
    SCREEN_SIZE = "Screen size", "屏幕尺寸";
    CANVAS_SQUARE = "Square", "正方形";
    A4_PORTRAIT = "A4 portrait", "A4 竖向";
    A4_LANDSCAPE = "A4 landscape", "A4 横向";
    FROM_A_FILE = "From a file", "从文件";
    OPEN_FILE = "Open OpenPaint file…", "打开 OpenPaint 文件…";
    LINES_AND_COLOURS = "Lines and colours", "线条与颜色";
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
}

/// What the gallery's notice says.
pub(crate) const fn gallery_notice(notice: GalleryNotice) -> Option<Text> {
    Some(match notice {
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
        Language::Chinese => format!("没有标题包含“{search}”。"),
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
    let topic = LINES_AND_COLOURS.get(language);
    match language {
        Language::English => format!("Stage {stage} · {topic}"),
        Language::Chinese => format!("第 {stage} 阶段 · {topic}"),
    }
}

/// The Today card's progress: sessions finished of about how many.
pub(crate) fn sessions_done(language: Language, done: u32, about: u32) -> String {
    match language {
        Language::English => format!("{done} of about {about} sessions done"),
        Language::Chinese => format!("已完成 {done} 节，共约 {about} 节"),
    }
}

/// A kept version's date and time of day: "5 Oct 2026 14:32", or "2026年10月5日 14:32";
/// `second` with the seconds.
pub(crate) fn kept_date(
    language: Language,
    (year, month, day): (i64, usize, i64),
    (hour, minute, second): (i64, i64, Option<i64>),
) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let time = match second {
        Some(second) => format!("{hour:02}:{minute:02}:{second:02}"),
        None => format!("{hour:02}:{minute:02}"),
    };
    match language {
        Language::English => format!("{day} {} {year} {time}", MONTHS[month - 1]),
        Language::Chinese => format!("{year}年{month}月{day}日 {time}"),
    }
}
