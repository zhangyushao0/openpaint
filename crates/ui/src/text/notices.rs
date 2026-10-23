//! What the canvas's notice pill says: saving and opening, the display reset, refused and failed
//! fills, transforms and adjustments, exports and pictures; the user-facing form of the app's
//! typed errors.

use openpaint_app::{Delivery, ExportError, ImageError, LayerRefusal, TransformRefusal};

use super::{Language, Text};

texts! { NOTICES;
    DISPLAY_RESET = "The display was reset.", "显示已重置。";
    SAVE_FAILED = "Couldn't save. Your drawing is still here.", "无法保存，你的画还在这里。";
    SAVING = "Saving…", "正在保存…";
    LAST_DRAWING_FAILED = "Couldn't open the last drawing. This is a new one.",
        "无法打开上次的画，这是一张新画。";
    DRAW_IN_FRAME = "Draw in the empty frame.", "请在空白框里画。";
    PICK_AFTER_COMPARE = "The original can be picked after Compare.", "叠图对比之后才能吸取原图的颜色。";
    SESSION_DONE = "This session is done.", "这次练习已完成。";
    DROP_IN_FRAME = "Drop inside the empty frame to fill.", "拖到空白框里即可填充。";
    DROP_IN_SELECTION = "Drop inside the selection to fill.", "拖到选区里即可填充。";
    FILL_FAILED = "Couldn't fill there. Try again.", "无法在这里填充，请重试。";
    NOTHING_TO_TRANSFORM = "Nothing to transform here.", "这里没有可变换的内容。";
    NOTHING_PASTED = "Nothing to paste: the copy holds no paint.", "没有可粘贴的内容：拷贝里没有颜料。";
    LIFT_FAILED = "Couldn't pick up the layer. Try again.", "无法拾取图层，请重试。";
    MASK_NOT_TRANSFORMED = "A mask can't be transformed. Tap the layer to move its paint.",
        "蒙版不能变换。轻点图层来移动它的内容。";
    NO_ROOM = "Can't insert: no room for another layer.", "无法插入：图层数量已达上限。";
    OPENING_IMAGE = "Opening the image…", "正在打开图像…";
    SAVE_DIALOG_FAILED = "Couldn't open the save dialog. Nothing was saved.",
        "无法打开保存对话框，未保存任何内容。";
    RENDER_FAILED = "Couldn't render the artwork. Nothing was saved.", "无法渲染作品，未保存任何内容。";
}

/// What the active layer refused to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refused {
    Fill,
    Transform,
    Adjust,
}

/// Why the active layer took no fill, transform or adjustment ("Can't fill: the layer is
/// hidden.").
pub(crate) fn refused(language: Language, verb: Refused, refusal: LayerRefusal) -> String {
    let hidden = refusal == LayerRefusal::Hidden;
    match language {
        Language::English => {
            let verb = match verb {
                Refused::Fill => "fill",
                Refused::Transform => "transform",
                Refused::Adjust => "adjust",
            };
            let why = if hidden { "hidden" } else { "locked" };
            format!("Can't {verb}: the layer is {why}.")
        }
        Language::Chinese => {
            let verb = match verb {
                Refused::Fill => "填充",
                Refused::Transform => "变换",
                Refused::Adjust => "调整",
            };
            let why = if hidden { "隐藏" } else { "锁定" };
            format!("无法{verb}：图层已{why}。")
        }
    }
}

/// Why the Transform tool did not turn on, but for the layer's refusals.
pub(crate) const fn transform_refused(refusal: TransformRefusal) -> Option<Text> {
    Some(match refusal {
        TransformRefusal::Layer(_) => return None,
        TransformRefusal::Empty => NOTHING_TO_TRANSFORM,
        TransformRefusal::NothingPasted => NOTHING_PASTED,
        TransformRefusal::Failed => LIFT_FAILED,
        TransformRefusal::Mask => MASK_NOT_TRANSFORMED,
        TransformRefusal::Full => NO_ROOM,
    })
}

/// An export at work: "Saving PNG…".
pub(crate) fn exporting(language: Language, delivery: Delivery, format: &str) -> String {
    match (language, delivery) {
        (Language::English, Delivery::Save) => format!("Saving {format}…"),
        (Language::English, Delivery::Share) => format!("Preparing {format}…"),
        (Language::Chinese, Delivery::Save) => format!("正在保存 {format}…"),
        (Language::Chinese, Delivery::Share) => format!("正在准备 {format}…"),
    }
}

/// An export written where the user chose.
pub(crate) fn exported(language: Language, name: &str) -> String {
    match language {
        Language::English => format!("Saved “{name}”."),
        Language::Chinese => format!("已保存“{name}”。"),
    }
}

/// Why an export of `format` failed.
pub(crate) fn export_failed(
    language: Language,
    (format, delivery): (&str, Delivery),
    error: &ExportError,
) -> String {
    let english = language == Language::English;
    match (error, delivery) {
        (ExportError::Picker(_), Delivery::Save) => SAVE_DIALOG_FAILED.get(language).to_owned(),
        (ExportError::Render, _) => RENDER_FAILED.get(language).to_owned(),
        (ExportError::Picker(_) | ExportError::Write(_), Delivery::Share) if english => {
            format!("Couldn't prepare the {format} to share.")
        }
        (ExportError::Picker(_) | ExportError::Write(_), Delivery::Share) => {
            format!("无法准备要分享的 {format}。")
        }
        (ExportError::Encode, _) if english => format!("Couldn't make the {format} file."),
        (ExportError::Encode, _) => format!("无法生成 {format} 文件。"),
        (ExportError::Write(_), Delivery::Save) if english => {
            format!("Couldn't write the {format} file. Is there space left?")
        }
        (ExportError::Write(_), Delivery::Save) => {
            format!("无法写入 {format} 文件。存储空间是否已满？")
        }
    }
}

/// Why a reference picture (`insert` false) or a picture to insert could not be used.
pub(crate) fn image_failed(language: Language, insert: bool, error: &ImageError) -> String {
    let (english, chinese) = match error {
        ImageError::Picker(_) => ("the picker failed.", "选择器出错。"),
        ImageError::Read(_) => ("the file could not be read.", "无法读取文件。"),
        ImageError::Unsupported => (
            "only PNG and JPEG images can be used.",
            "只能使用 PNG 和 JPEG 图像。",
        ),
        ImageError::Damaged => ("the file is damaged.", "文件已损坏。"),
        ImageError::TooLarge => ("it is larger than 50 megapixels.", "图像超过 5000 万像素。"),
    };
    match (language, insert) {
        (Language::English, false) => format!("Couldn't open the image: {english}"),
        (Language::English, true) => format!("Couldn't insert the image: {english}"),
        (Language::Chinese, false) => format!("无法打开图像：{chinese}"),
        (Language::Chinese, true) => format!("无法插入图像：{chinese}"),
    }
}
