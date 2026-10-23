//! The layer panel: its header, rows, options and masks.

use openpaint_core::{BlendMode, Unmergeable};

use super::Text;

texts! { LAYERS_TABLE;
    ADD_LAYER = "Add layer", "添加图层";
    NEW_GROUP = "New group", "新建组";
    RENAME_LAYER = "Rename layer", "重命名图层";
    ALPHA_LOCK = "Alpha Lock", "阿尔法锁定";
    CLIPPING_MASK = "Clipping Mask", "剪辑蒙版";
    ADD_MASK = "Add Mask", "添加蒙版";
    MERGE_DOWN = "Merge Down", "向下合并";
    GROUP = "Group", "编组";
    UNGROUP = "Ungroup", "取消编组";
    DELETE_LAYER = "Delete layer", "删除图层";
    DUPLICATE_LAYER = "Duplicate layer", "复制图层";
    GROUP_OPACITY = "Group opacity", "组不透明度";
    LAYER_MASK = "Layer Mask", "图层蒙版";
    MASK_OFF = "Off", "已关闭";
    MASK_INVERTED = "Inverted", "已反转";
    TURN_MASK_OFF = "Turn mask off", "关闭蒙版";
    TURN_MASK_ON = "Turn mask on", "开启蒙版";
    BLACK_HIDES = "Black hides · white shows", "黑色隐藏 · 白色显示";
    DELETE_MASK = "Delete mask", "删除蒙版";
    HIDE_GROUP = "Hide group", "隐藏组";
    SHOW_GROUP = "Show group", "显示组";
    HIDE_LAYER = "Hide layer", "隐藏图层";
    SHOW_LAYER = "Show layer", "显示图层";
}

texts! { LAYER_WORDS;
    NORMAL = "Normal", "正常";
    MULTIPLY = "Multiply", "正片叠底";
    BLEND_SCREEN = "Screen", "滤色";
    OVERLAY = "Overlay", "叠加";
    NOTHING_BELOW = "Nothing below", "下方无图层";
    HIDDEN_LAYER = "Hidden layer", "图层已隐藏";
    LOCKED_LAYER = "Locked layer", "图层已锁定";
    MODES_CLASH = "Modes clash", "混合模式冲突";
    BELOW_CLIPS = "Below clips", "下方有剪辑";
    HAS_CLIPS = "Has clips", "带有剪辑";
    TOO_DEEP = "Too deep", "嵌套过深";
}

/// A blend mode's name.
pub(crate) const fn blend(blend: BlendMode) -> Text {
    match blend {
        BlendMode::Normal => NORMAL,
        BlendMode::Multiply => MULTIPLY,
        BlendMode::Screen => BLEND_SCREEN,
        BlendMode::Overlay => OVERLAY,
    }
}

/// Why Merge Down is grey, at the end of its row.
pub(crate) const fn unmergeable(why: Unmergeable) -> Text {
    match why {
        Unmergeable::NothingBelow => NOTHING_BELOW,
        Unmergeable::Hidden => HIDDEN_LAYER,
        Unmergeable::Locked => LOCKED_LAYER,
        Unmergeable::Modes => MODES_CLASH,
        Unmergeable::BelowClips => BELOW_CLIPS,
        Unmergeable::ClippedTo => HAS_CLIPS,
        Unmergeable::TooDeep => TOO_DEEP,
    }
}
