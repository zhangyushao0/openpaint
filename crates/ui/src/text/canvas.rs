//! The canvas's chrome: the islands, the side bar, Actions and its Pen buttons page, the
//! adjustments, and the bars of the selection, the transform, QuickShape, copy and paste and the
//! fill.

use openpaint_app::{ButtonAction, ClipboardAction, PenButton, Symmetry};
use openpaint_core::ToneChannel;

use super::{Language, Text};

texts! { CANVAS;
    GALLERY = "Gallery", "图库";
    ACTIONS = "Actions", "操作";
    ADJUSTMENTS = "Adjustments", "调整";
    SELECTION = "Selection", "选区";
    TRANSFORM = "Transform", "变换";
    BRUSHES = "Brushes", "笔刷";
    SMUDGE = "Smudge", "涂抹";
    ERASER = "Eraser", "橡皮擦";
    LAYERS = "Layers", "图层";
    COLOUR = "Colour", "颜色";
    BRUSH_SIZE = "Brush size", "笔刷大小";
    BRUSH_OPACITY = "Brush opacity", "笔刷不透明度";
    SMUDGE_SIZE = "Smudge size", "涂抹大小";
    SMUDGE_STRENGTH = "Strength", "强度";
    EYEDROPPER = "Eyedropper", "吸管";
    ADD = "Add", "添加";
    INSERT_IMAGE = "Insert image…", "插入图像…";
    AS_A_NEW_LAYER = "As a new layer", "作为新图层";
    COPY_AND_PASTE = "Copy & Paste", "拷贝与粘贴";
    THREE_FINGERS_DOWN = "Three fingers down", "三指下滑";
    SHARE = "Share", "分享";
    SAVE_PNG = "Save PNG…", "保存 PNG…";
    SAVE_JPEG = "Save JPEG…", "保存 JPEG…";
    SHARE_PNG = "Share PNG…", "分享 PNG…";
    SHARE_JPEG = "Share JPEG…", "分享 JPEG…";
    LOSSLESS = "Lossless", "无损";
    SMALLER = "Smaller", "更小";
    UP_TO_2X = "Up to 2×", "最高 2×";
    CANVAS_SECTION = "Canvas", "画布";
    REFERENCE = "Reference", "参考";
    CHOOSE_REFERENCE = "Choose reference image…", "选择参考图像…";
    CLOSE_REFERENCE = "Close reference", "关闭参考";
    DRAWING_GUIDE = "Drawing Guide", "绘图指引";
    MOVE_THE_CENTRE = "A finger moves the centre.", "用手指移动中心。";
    PEN = "Pen", "触控笔";
    PEN_BUTTONS = "Pen buttons", "笔按键";
    FOR_YOUR_PEN = "For the buttons your pen has.", "按你的笔上有的按键设置。";
    ERASER_END = "Eraser end", "橡皮擦端";
    ERASER_END_ERASES = "Erases with the eraser's brush.", "用橡皮擦的笔刷擦除。";
    WHILE_HELD = "Eraser, Eyedropper: while held.", "橡皮擦、吸管：按住时生效。";
    LANGUAGE = "Language", "语言";
    HUE_SATURATION_BRIGHTNESS = "Hue, Saturation, Brightness", "色相、饱和度、亮度";
    CURVES = "Curves", "曲线";
    GAUSSIAN_BLUR = "Gaussian Blur", "高斯模糊";
    SHARPEN = "Sharpen", "锐化";
    BRIGHTNESS = "Brightness", "亮度";
    RADIUS = "Radius", "半径";
    AMOUNT = "Amount", "数量";
    CURVE = "Curve", "曲线";
    FREEHAND = "Freehand", "手绘";
    RECTANGLE = "Rectangle", "矩形";
    CLOSE_PATH = "Close", "闭合";
    DESELECT = "Deselect", "取消选择";
    FLIP_HORIZONTAL = "Flip Horizontal", "水平翻转";
    FLIP_VERTICAL = "Flip Vertical", "垂直翻转";
    ROTATE_45 = "Rotate 45°", "旋转 45°";
    EDIT_SHAPE = "Edit shape", "编辑形状";
    LIFT_TO_FILL = "Lift to fill. Hold still to adjust.", "抬起即填充，按住不动可调整。";
    FILLED = "Filled.", "已填充。";
    CONTINUE_FILLING = "Continue filling", "继续填充";
    TAP_TO_FILL = "Tap an area to fill it.", "轻点区域即可填充。";
}

texts! { CANVAS_WORDS;
    GAMMA = "Gamma", "伽马";
    RED = "Red", "红";
    GREEN = "Green", "绿";
    BLUE = "Blue", "蓝";
    VERTICAL = "Vertical", "垂直";
    HORIZONTAL = "Horizontal", "水平";
    QUADRANT = "Quadrant", "四象限";
    RADIAL = "Radial", "径向";
    CUT = "Cut", "剪切";
    COPY = "Copy", "拷贝";
    COPY_ALL = "Copy All", "拷贝全部";
    CUT_AND_PASTE = "Cut & Paste", "剪切并粘贴";
    PASTE = "Paste", "粘贴";
    SIDE_BUTTON_1 = "Side button 1", "侧键 1";
    SIDE_BUTTON_2 = "Side button 2", "侧键 2";
    SIDE_BUTTON_3 = "Side button 3", "侧键 3";
    NOTHING = "Nothing", "无";
    SWITCH_TOOL = "Switch tool", "切换工具";
}

/// The curves the Curves panel offers, with Procreate's names.
pub(crate) const fn channel(channel: ToneChannel) -> Text {
    match channel {
        ToneChannel::Composite => GAMMA,
        ToneChannel::Red => RED,
        ToneChannel::Green => GREEN,
        ToneChannel::Blue => BLUE,
    }
}

/// The Drawing Guide's symmetries.
pub(crate) const fn symmetry(symmetry: Symmetry) -> Text {
    match symmetry {
        Symmetry::Vertical => VERTICAL,
        Symmetry::Horizontal => HORIZONTAL,
        Symmetry::Quadrant => QUADRANT,
        Symmetry::Radial => RADIAL,
    }
}

/// The copy menu's commands.
pub(crate) const fn clipboard(action: ClipboardAction) -> Text {
    match action {
        ClipboardAction::Cut => CUT,
        ClipboardAction::Copy => COPY,
        ClipboardAction::CopyAll => COPY_ALL,
        ClipboardAction::Duplicate => super::DUPLICATE,
        ClipboardAction::CutPaste => CUT_AND_PASTE,
        ClipboardAction::Paste => PASTE,
    }
}

/// The pen's side buttons, from the tip outwards.
pub(crate) const fn pen_button(button: PenButton) -> Text {
    match button {
        PenButton::Primary => SIDE_BUTTON_1,
        PenButton::Secondary => SIDE_BUTTON_2,
        PenButton::Tertiary => SIDE_BUTTON_3,
    }
}

/// What a side button can do.
pub(crate) const fn button_action(action: ButtonAction) -> Text {
    match action {
        ButtonAction::Nothing => NOTHING,
        ButtonAction::Eraser => ERASER,
        ButtonAction::Eyedropper => EYEDROPPER,
        ButtonAction::Undo => super::UNDO,
        ButtonAction::Redo => super::REDO,
        ButtonAction::SwitchTool => SWITCH_TOOL,
    }
}

/// The fill bar while the threshold is adjusted.
pub(crate) fn threshold(language: Language, threshold: f32) -> String {
    let percent = threshold * 100.0;
    match language {
        Language::English => format!("Threshold {percent:.1} % · slide to adjust"),
        Language::Chinese => format!("阈值 {percent:.1} % · 滑动调整"),
    }
}
