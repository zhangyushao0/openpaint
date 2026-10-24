//! Words many screens share: answers, commands, the language's own names and the version line.

use super::{Language, Text};

texts! { COMMON;
    UNDO = "Undo", "撤销";
    REDO = "Redo", "重做";
    DONE = "Done", "完成";
    CANCEL = "Cancel", "取消";
    BACK = "Back", "返回";
    CLOSE = "Close", "关闭";
    RESET = "Reset", "重置";
    DELETE = "Delete", "删除";
    RENAME = "Rename", "重命名";
    DUPLICATE = "Duplicate", "复制";
    CLEAR = "Clear", "清除";
    INVERT = "Invert", "反转";
    APPLY = "Apply", "应用";
    EDIT = "Edit", "编辑";
    OK = "OK", "确定";
    RETRY = "Retry", "重试";
    NAME = "Name", "名称";
    NONE = "None", "无";
    OPACITY = "Opacity", "不透明度";
    HUE = "Hue", "色相";
    SATURATION = "Saturation", "饱和度";
    // Oklab's lightness and chroma as the colour table and the radar name them (V大's 明度, 纯度).
    LIGHTNESS = "Lightness", "明度";
    CHROMA = "Chroma", "纯度";
    // The language's own name, the same in both.
    ENGLISH = "English", "English";
    CHINESE = "中文", "中文";
}

/// Each language as Actions offers it, in its own words.
#[must_use]
pub(crate) const fn language_name(language: Language) -> Text {
    match language {
        Language::English => ENGLISH,
        Language::Chinese => CHINESE,
    }
}

/// Actions' last line: the app's version and its storage format.
#[must_use]
pub(crate) fn version_line(language: Language, version: &str, format: u32) -> String {
    match language {
        Language::English => format!("OpenPaint {version} · format {format}"),
        Language::Chinese => format!("OpenPaint {version} · 格式 {format}"),
    }
}
