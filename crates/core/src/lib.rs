//! Artwork semantics: the only owner of layers, strokes, commands and their validation.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
//! `lib.rs` only declares modules and re-exports; concepts live in their own files.
#![forbid(unsafe_code)]

mod artwork;
mod brush;
mod command;
mod error;
mod id;
mod layer;
mod layer_command;
mod parts;
mod session;
mod storage;
mod stroke;
mod stroke_list;
mod value;

pub use artwork::{Artwork, MAX_LAYERS};
pub use brush::{
    BrushRecipe, Curve, Dynamics, Grain, MAX_CURVE_POINTS, MAX_GRAIN_SCALE, MAX_SPEED,
};
pub use command::{Applied, ArtworkCommand, Change, LayerCommand};
pub use error::CoreError;
pub use id::{LayerId, MAX_ID, StrokeId};
pub use layer::{Layer, LayerProps};
pub use parts::{ArtworkParts, LayerParts, RecipeParts, SampleParts, StrokeParts};
pub use session::{ActiveStroke, StrokeSession};
pub use stroke::{MAX_BRUSH_SIZE, MAX_STROKE_SAMPLES, Stroke, StrokeSample, StrokeStyle};
pub use stroke_list::StrokeList;
pub use value::{
    COORD_LIMIT, CanvasPoint, CanvasSize, Color, LayerName, MAX_CANVAS_SIDE, MAX_NAME_CHARS,
};
