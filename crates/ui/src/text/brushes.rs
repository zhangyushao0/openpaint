//! The brush picker and the brush editor: sets, the built-in brushes' names, the settings and
//! the dynamics pages.

use openpaint_app::{Axis, BrushSet, Dynamic, Knob};
use openpaint_core::{Rotation, ShapeTexture, Stamp};

use super::{Language, Text};

texts! { BRUSHES_TABLE;
    RECENT_EMPTY = "Brushes you draw with appear here.", "用过的笔刷会出现在这里。";
    STREAMLINE = "StreamLine", "流线";
    RENAME_BRUSH = "Rename brush", "重命名笔刷";
    STROKES_STAY = "Strokes drawn with it stay.", "用它画过的笔画会保留。";
    SET = "Set", "分组";
    STROKE = "Stroke", "笔画";
    SHAPE = "Shape", "形状";
    GRAIN = "Grain", "颗粒";
    SIZE = "Size", "大小";
    DYNAMICS = "Dynamics", "动态";
    PAINT = "Paint", "颜料";
    COVERING = "Covering", "覆盖";
    BUILDS_UP = "Builds up", "累积";
    TIP = "Tip", "笔尖";
    TIP_FLAT = "Flat", "扁平";
    BELL = "Bell", "钟形";
    PAPER = "Paper", "纸纹";
    SKETCH_PAPER = "Sketch", "素描纸";
    PAPER_CONTINUOUS = "Paper keeps the stroke continuous.", "有纸纹时笔画保持连续。";
    ROTATION = "Rotation", "旋转";
    FIXED = "Fixed", "固定";
    DIRECTION = "Direction", "方向";
    AZIMUTH = "Azimuth", "方位角";
    CONTINUOUS = "Continuous", "连续";
    SIZE_DYNAMICS = "Size dynamics", "大小动态";
    OPACITY_DYNAMICS = "Opacity dynamics", "不透明度动态";
    LIGHT = "Light", "轻";
    FIRM = "Firm", "重";
    SLOW = "Slow", "慢";
    UPRIGHT = "Upright", "竖直";
    LYING_FLAT = "Flat", "平躺";
    PREVIEW = "Preview", "预览";
}

texts! { BRUSH_WORDS;
    RECENT = "Recent", "最近";
    SKETCHING = "Sketching", "素描";
    INKING = "Inking", "上墨";
    PAINTING = "Painting", "绘画";
    TEXTURE = "Texture", "纹理";
    DISC = "Disc", "圆形";
    STAMP_SQUARE = "Square", "方形";
    ELLIPSE = "Ellipse", "椭圆";
    CHALK = "Chalk", "粉笔";
    GRAINY = "Grainy", "颗粒";
    SPLATTER = "Splatter", "飞溅";
    ROUGH_SQUARE = "Rough square", "粗糙方形";
    SPACING = "Spacing", "间距";
    TAPER_START = "Taper start", "起笔渐细";
    TAPER_END = "Taper end", "收笔渐细";
    TAPER_TIP = "Taper tip", "渐细笔尖";
    ASPECT = "Aspect", "纵横比";
    ANGLE = "Angle", "角度";
    ROTATION_JITTER = "Rotation jitter", "旋转抖动";
    SIZE_JITTER = "Size jitter", "大小抖动";
    SCATTER = "Scatter", "散布";
    SOFTNESS = "Softness", "柔和度";
    GRAIN_STRENGTH = "Strength", "强度";
    GRAIN_SCALE = "Scale", "缩放";
    FLOW = "Flow", "流量";
    MAX_DENSITY = "Max density", "最大浓度";
    MINIMUM = "Minimum", "最小";
    MAXIMUM = "Maximum", "最大";
    PRESSURE = "Pressure", "压力";
    SPEED = "Speed", "速度";
    TILT = "Tilt", "倾斜";
}

texts! { BRUSH_NAMES;
    PENCIL = "Pencil", "铅笔";
    INK_PEN = "Ink Pen", "墨水笔";
    INKER = "Inker", "勾线笔";
    MONOLINE = "Monoline", "单线笔";
    HARD_ROUND = "Hard Round", "硬圆笔";
    SOFT_BRUSH = "Soft Brush", "柔边笔刷";
    FLAT_BRUSH = "Flat", "平头笔";
    OPAQUE_PAINT = "Opaque Paint", "不透明颜料";
    TEXTURED_ROUND = "Textured Round", "纹理圆笔";
    SPECKLE = "Speckle", "斑点笔";
}

/// The built-in brushes' names, as their files give them.
pub(crate) const BUILT_IN: [Text; 10] = [
    PENCIL,
    INK_PEN,
    INKER,
    MONOLINE,
    HARD_ROUND,
    SOFT_BRUSH,
    FLAT_BRUSH,
    OPAQUE_PAINT,
    TEXTURED_ROUND,
    SPECKLE,
];

/// A set's word in the pickers and the editor.
pub(crate) const fn brush_set(set: BrushSet) -> Text {
    match set {
        BrushSet::Recent => RECENT,
        BrushSet::Sketching => SKETCHING,
        BrushSet::Inking => INKING,
        BrushSet::Painting => PAINTING,
        BrushSet::Texture => TEXTURE,
    }
}

/// A stamp's shape.
pub(crate) const fn stamp(stamp: Stamp) -> Text {
    match stamp {
        Stamp::Disc => DISC,
        Stamp::Square => STAMP_SQUARE,
        Stamp::Ellipse => ELLIPSE,
        Stamp::Texture(ShapeTexture::Chalk) => CHALK,
        Stamp::Texture(ShapeTexture::Grainy) => GRAINY,
        Stamp::Texture(ShapeTexture::Splatter) => SPLATTER,
        Stamp::Texture(ShapeTexture::RoughSquare) => ROUGH_SQUARE,
    }
}

/// Which way stamps turn.
pub(crate) const fn rotation(rotation: Rotation) -> Text {
    match rotation {
        Rotation::Fixed => FIXED,
        Rotation::Direction => DIRECTION,
        Rotation::Azimuth => AZIMUTH,
    }
}

/// What a setting of the editor is called.
pub(crate) const fn knob(knob: Knob) -> Text {
    match knob {
        Knob::Spacing => SPACING,
        Knob::TaperStart => TAPER_START,
        Knob::TaperEnd => TAPER_END,
        Knob::TaperTip => TAPER_TIP,
        Knob::Aspect => ASPECT,
        Knob::Angle => ANGLE,
        Knob::RotationJitter => ROTATION_JITTER,
        Knob::SizeJitter => SIZE_JITTER,
        Knob::Scatter => SCATTER,
        Knob::Softness => SOFTNESS,
        Knob::GrainStrength => GRAIN_STRENGTH,
        Knob::GrainScale => GRAIN_SCALE,
        Knob::Flow => FLOW,
        Knob::MaxDensity => MAX_DENSITY,
        Knob::SizeMin => MINIMUM,
        Knob::SizeMax => MAXIMUM,
    }
}

/// What a dynamics curve follows.
pub(crate) const fn axis(axis: Axis) -> Text {
    match axis {
        Axis::Pressure => PRESSURE,
        Axis::Speed => SPEED,
        Axis::Tilt => TILT,
    }
}

/// A dynamics page's heading.
pub(crate) const fn dynamic(dynamic: Dynamic) -> Text {
    match dynamic {
        Dynamic::Size => SIZE_DYNAMICS,
        Dynamic::Opacity => OPACITY_DYNAMICS,
    }
}

/// A length in brush diameters ("1.5 × size") or radii ("0.5 × radius").
pub(crate) fn times(language: Language, value: f32, radius: bool) -> String {
    let unit = match (language, radius) {
        (Language::English, false) => "size",
        (Language::English, true) => "radius",
        (Language::Chinese, false) => "大小",
        (Language::Chinese, true) => "半径",
    };
    format!("{value:.1} × {unit}")
}

/// The most a curve's factor may rise, under its square.
pub(crate) fn up_to(language: Language, top: f32) -> String {
    match language {
        Language::English => format!("Up to {top:.0}×"),
        Language::Chinese => format!("最高 {top:.0}×"),
    }
}

/// The question before a copy of a brush is deleted.
pub(crate) fn delete_brush(language: Language, name: &str) -> String {
    match language {
        Language::English => format!("Delete {name}?"),
        Language::Chinese => format!("删除“{name}”？"),
    }
}
