//! Artwork semantics: the only owner of layers, strokes, commands and their validation.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
//! `lib.rs` only declares modules and re-exports; concepts live in their own files.
#![forbid(unsafe_code)]

mod adjust;
mod artwork;
mod blend;
mod brush;
mod clear;
mod command;
mod copy;
mod deposit;
mod error;
mod filter;
mod group;
mod group_command;
mod id;
mod layer;
mod layer_command;
mod layer_moves;
mod mask;
mod mask_command;
mod mask_ops;
mod merge;
mod nested;
mod parts;
mod parts_recipe;
mod region;
mod session;
mod storage;
mod stroke;
mod stroke_list;
mod tone;
mod transform;
mod value;

pub use adjust::{
    AdjustEdit, AdjustParts, Adjustment, AdjustmentKind, AdjustmentParts, Curves, Hsb, ToneChannel,
};
pub use artwork::{Artwork, MAX_LAYERS};
pub use blend::BlendMode;
pub use brush::{
    BrushRecipe, Curve, Dynamics, Grain, MAX_CURVE_POINTS, MAX_GRAIN_SCALE, MAX_SPEED,
};
pub use clear::ClearEdit;
pub use command::{Applied, ArtworkCommand, Change, GroupCommand, LayerCommand, MaskCommand};
pub use copy::Copied;
pub use deposit::{
    Accumulation, Deposit, MAX_FLOW, MAX_PAPER_DEPTH, PAPER_SCALES, Paper, PaperTexture, Tip,
};
pub use error::CoreError;
pub use filter::{Blur, Sharpen};
pub use group::LayerGroup;
pub use id::{GroupId, LayerId, MAX_ID, StrokeId};
pub use layer::{Layer, LayerProps};
pub use layer_moves::{LayerPlace, layer_places};
pub use mask::{LayerMask, grey_of};
pub use merge::Unmergeable;
pub use nested::{MAX_NESTING, NestedEdit, NestedLayer};
pub use parts::{
    AccumulationParts, ArtworkParts, ClearParts, EditParts, FillParts, GroupParts, LayerMaskParts,
    LayerParts, MaskParts, NestedParts, PaperParts, PatchParts, RecipeParts, SampleParts,
    StrokeParts, TransformParts,
};
pub use region::{FillEdit, RegionEdit, RegionMask};
pub use session::{ActiveStroke, StrokeSession};
pub use stroke::{
    MAX_BRUSH_SIZE, MAX_STROKE_SAMPLES, Stroke, StrokeBlend, StrokeSample, StrokeStyle, StrokeTool,
};
pub use stroke_list::StrokeList;
pub use tone::{MAX_TONE_POINTS, TONE_GAP, ToneCurve};
pub use transform::{MAX_SCALE, Patch, Placement, TransformEdit};
pub use value::{
    COORD_LIMIT, CanvasPoint, CanvasSize, Color, LayerName, MAX_CANVAS_SIDE, MAX_NAME_CHARS,
};
